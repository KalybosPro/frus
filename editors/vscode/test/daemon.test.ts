// The daemon client: answers matched to their questions, events and messages handed on.

import assert from "node:assert/strict";
import { EventEmitter } from "node:events";
import { PassThrough } from "node:stream";
import { test } from "node:test";

import { DaemonMessage, Devices, isSimulated } from "../src/daemon";

/** A `Devices` talking to a stand-in daemon: what it is sent, and a way to answer. */
function connected() {
  const devices = new Devices();
  const process = new EventEmitter() as any;
  process.stdin = new PassThrough();
  const sent: any[] = [];
  process.stdin.on("data", (chunk: Buffer) =>
    chunk.toString().split("\n").filter(Boolean).forEach((line) => sent.push(JSON.parse(line))),
  );
  (devices as any).process = process;
  return { devices, process, sent };
}

const tick = () => new Promise((resolve) => setImmediate(resolve));

test("an answer goes to the question it answers", async () => {
  const { devices, sent } = connected();
  const emulators = devices.emulators();
  const create = devices.createEmulator("pixel");
  await tick();
  assert.deepEqual(
    sent.map((s) => [s.method, s.params]),
    [
      ["emulator.getEmulators", {}],
      ["emulator.create", { name: "pixel" }],
    ],
  );
  // Answered out of order.
  devices.take({ id: sent[1].id, result: { success: true, emulatorName: "pixel", error: null } });
  devices.take({ id: sent[0].id, result: [{ id: "pixel", name: "pixel", category: "mobile", platformType: "android" }] });
  assert.equal((await create).emulatorName, "pixel");
  assert.equal((await emulators)[0].platformType, "android");
});

test("an error answer rejects", async () => {
  const { devices, sent } = connected();
  const launch = devices.launchEmulator("nope");
  await tick();
  devices.take({ id: sent[0].id, error: "emulator 'nope' not found" });
  await assert.rejects(launch, /not found/);
});

test("the daemon's messages and device events are handed on", () => {
  const { devices } = connected();
  const said: DaemonMessage[] = [];
  let changes = 0;
  devices.onMessage((m) => said.push(m));
  devices.onDidChange(() => changes++);
  devices.take({ event: "daemon.logMessage", params: { level: "status", message: "Creating…" } });
  devices.take({ event: "daemon.showMessage", params: { level: "error", title: "frus", message: "No SDK" } });
  devices.take({ event: "device.added", params: { id: "emulator-5554", name: "pixel", category: "mobile" } });
  assert.deepEqual(
    said.map((m) => [m.kind, m.level, m.message]),
    [
      ["log", "status", "Creating…"],
      ["show", "error", "No SDK"],
    ],
  );
  assert.equal(changes, 1);
  assert.equal(devices.get("emulator-5554")?.name, "pixel");
});

test("a simulated phone is told apart from this computer", () => {
  // This computer: its id is its platform.
  assert.equal(isSimulated({ id: "windows", platform: "windows", category: "desktop" }), false);
  // A built-in profile and a project's own device: this computer's platform, an id of their own.
  assert.equal(isSimulated({ id: "iphone-15", platform: "windows", category: "desktop" }), true);
  assert.equal(isSimulated({ id: "kiosk-10", platform: "macos", category: "desktop" }), true);
  // Phones, emulators and browsers are not.
  assert.equal(isSimulated({ id: "emulator-5554", platform: "android", category: "mobile" }), false);
  assert.equal(isSimulated({ id: "edge", platform: "web", category: "web" }), false);
});
