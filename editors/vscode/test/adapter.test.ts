// The debug adapter, without VS Code: requests in, protocol messages out, and a `frus` process
// that is either a fake (streams) or the real program (FRUS_E2E_FRUS and FRUS_E2E_APP set).

import assert from "node:assert/strict";
import { EventEmitter } from "node:events";
import { PassThrough } from "node:stream";
import { test } from "node:test";

import { FrusDebugAdapter } from "../src/adapter";

/** A stand-in for `frus run --machine`: what it is sent, and what it says. */
function fakeFrus() {
  const process = new EventEmitter() as any;
  process.stdout = new PassThrough();
  process.stderr = new PassThrough();
  process.stdin = new PassThrough();
  process.exitCode = null;
  process.kill = () => process.emit("exit", 1);
  const sent: any[] = [];
  process.stdin.on("data", (chunk: Buffer) =>
    chunk
      .toString()
      .split("\n")
      .filter(Boolean)
      .forEach((line) => sent.push(JSON.parse(line))),
  );
  const say = (message: object) => process.stdout.write(JSON.stringify(message) + "\n");
  return { process, sent, say };
}

function session(spawned: { args?: string[] } = {}) {
  const frus = fakeFrus();
  const adapter = new FrusDebugAdapter((_program, args) => {
    spawned.args = args;
    return frus.process;
  });
  const messages: any[] = [];
  adapter.onDidSendMessage((m) => messages.push(m));
  let seq = 1;
  const request = (command: string, args: object = {}) =>
    adapter.handleMessage({ seq: seq++, type: "request", command, arguments: args });
  return { adapter, frus, messages, request };
}

const tick = () => new Promise((resolve) => setImmediate(resolve));
const outputs = (messages: any[], category: string) =>
  messages.filter((m) => m.event === "output" && m.body.category === category).map((m) => m.body.output);

test("initialize answers the capabilities, then says it is ready", () => {
  const { messages, request } = session();
  request("initialize");
  assert.equal(messages[0].type, "response");
  assert.equal(messages[0].body.supportsRestartRequest, true);
  assert.equal(messages[1].event, "initialized");
});

test("launch runs frus run --machine on the device, with its options", () => {
  const spawned: { args?: string[] } = {};
  const { messages, request } = session(spawned);
  request("launch", { frus: "frus", cwd: ".", deviceId: "edge", release: true, args: ["/x"] });
  assert.deepEqual(spawned.args, ["run", "--machine", "-d", "edge", "--release", "--", "/x"]);
  assert.equal(messages.find((m) => m.command === "launch").success, true);
});

test("a hot launch asks frus run for hot reload", () => {
  const spawned: { args?: string[] } = {};
  const { request } = session(spawned);
  request("launch", { frus: "frus", cwd: ".", hot: true });
  assert.deepEqual(spawned.args, ["run", "--machine", "--hot"]);
});

test("the application's log reaches the Debug Console, errors as errors", async () => {
  const { frus, messages, request } = session();
  request("launch", { frus: "frus", cwd: "." });
  frus.say({ event: "app.log", params: { appId: "a", log: "hello", error: false } });
  frus.say({ event: "app.log", params: { appId: "a", log: "it broke", error: true } });
  frus.say({ event: "app.progress", params: { appId: "a", message: "Building…" } });
  frus.process.stderr.write("   Compiling app v0.1.0\n");
  await tick();
  assert.deepEqual(outputs(messages, "stdout"), ["hello\n"]);
  assert.deepEqual(outputs(messages, "stderr"), ["it broke\n"]);
  assert.ok(outputs(messages, "console").includes("Building…\n"));
  assert.ok(outputs(messages, "console").includes("   Compiling app v0.1.0\n"));
});

test("reload, restart and stop become frus's commands", async () => {
  const { frus, messages, request } = session();
  request("launch", { frus: "frus", cwd: "." });
  request("hotReload");
  request("restart");
  request("disconnect");
  await tick();
  assert.deepEqual(
    frus.sent.map((c) => [c.method, c.params?.fullRestart]),
    [
      ["app.restart", false],
      ["app.restart", true],
      ["app.stop", undefined],
    ],
  );
  frus.say({ event: "app.stop", params: { appId: "a", error: null } });
  await tick();
  assert.equal(messages.filter((m) => m.event === "terminated").length, 1);
  // Once ended, nothing more is sent to it.
  request("hotReload");
  await tick();
  assert.equal(frus.sent.length, 3);
});

test("a run that ends on its own ends the session", async () => {
  const { frus, messages, request } = session();
  request("launch", { frus: "frus", cwd: "." });
  frus.process.emit("exit", 0);
  frus.process.emit("exit", 0);
  await tick();
  assert.equal(messages.filter((m) => m.event === "terminated").length, 1);
});

// The real thing: FRUS_E2E_FRUS=<frus.exe> FRUS_E2E_APP=<an application's directory>.
const frusExe = process.env.FRUS_E2E_FRUS;
const app = process.env.FRUS_E2E_APP;
test("end to end with the real frus", { skip: !frusExe || !app, timeout: 1_200_000 }, async () => {
  const adapter = new FrusDebugAdapter();
  const messages: any[] = [];
  adapter.onDidSendMessage((m) => messages.push(m));
  let seq = 1;
  const request = (command: string, args: object = {}) =>
    adapter.handleMessage({ seq: seq++, type: "request", command, arguments: args });
  const until = async (pred: () => boolean, ms: number) => {
    const end = Date.now() + ms;
    while (!pred() && Date.now() < end) await new Promise((r) => setTimeout(r, 200));
    assert.ok(pred(), "timed out");
  };
  const console_ = () => outputs(messages, "console").join("");

  request("initialize");
  request("launch", { frus: frusExe, cwd: app, deviceId: process.env.FRUS_E2E_DEVICE ?? "windows" });
  await until(() => console_().includes("The application is running"), 900_000);
  await new Promise((r) => setTimeout(r, 5000));
  request("hotReload");
  await until(() => /Reloaded in [\d.]+ s/.test(console_()), 600_000);
  request("disconnect");
  await until(() => messages.some((m) => m.event === "terminated"), 30_000);
  const logs = outputs(messages, "stdout").length + outputs(messages, "stderr").length;
  process.stdout.write(`# ${logs} application log lines reached the console\n`);
  assert.ok(logs > 0, "the application's log reached the Debug Console");
});

test("the other keys of the run go to it by their letter", async () => {
  const { frus, request } = session();
  request("launch", { frus: "frus", cwd: "." });
  request("key", { key: "p" });
  request("key", { key: "S" });
  request("key", { key: "pp" });
  request("key", {});
  await tick();
  assert.deepEqual(
    frus.sent.map((c) => [c.method, c.params?.key]),
    [
      ["app.key", "p"],
      ["app.key", "S"],
    ],
  );
});
