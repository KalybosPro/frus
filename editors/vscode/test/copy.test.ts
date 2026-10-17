// The copy the extension runs instead of the installed `frus`.

import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import * as fs from "node:fs";
import * as os from "node:os";
import * as path from "node:path";
import { test } from "node:test";

import { resolveProgram, runnableCopy, watchProgram } from "../src/copy";

function tempDir(): string {
  return fs.mkdtempSync(path.join(os.tmpdir(), "frus-copy-"));
}

test("a bare name is found on the PATH, with Windows' extensions", () => {
  const bin = tempDir();
  fs.writeFileSync(path.join(bin, "frus.exe"), "");
  const env = { PATH: ["/nowhere", bin].join(";"), PATHEXT: ".COM;.EXE" };
  assert.equal(resolveProgram("frus", env, "win32"), path.join(bin, "frus.exe"));
  assert.equal(resolveProgram("other", env, "win32"), undefined);

  // A path is taken as it is.
  fs.writeFileSync(path.join(bin, "frus"), "");
  assert.equal(resolveProgram(path.join(bin, "frus"), {}, "linux"), path.join(bin, "frus"));
  // On Unix, `:` separates the PATH (where a Windows path, `D:\...`, cannot be on it).
  if (process.platform !== "win32") {
    assert.equal(resolveProgram("frus", { PATH: `/nowhere:${bin}` }, "linux"), path.join(bin, "frus"));
  }
});

test("one copy per install, and the older ones go", () => {
  const dir = tempDir();
  const installed = path.join(dir, "frus.exe");
  const copies = path.join(dir, "copies");
  fs.writeFileSync(installed, "version 1");
  const first = runnableCopy(installed, copies);
  assert.equal(fs.readFileSync(first, "utf8"), "version 1");
  assert.equal(runnableCopy(installed, copies), first, "the same install, the same copy");

  fs.writeFileSync(installed, "version 2, longer");
  fs.utimesSync(installed, new Date(), new Date(Date.now() + 5000));
  const second = runnableCopy(installed, copies);
  assert.notEqual(second, first);
  assert.equal(fs.readFileSync(second, "utf8"), "version 2, longer");
  assert.deepEqual(fs.readdirSync(copies), [path.basename(second)]);
});

test("a replaced program is noticed", async () => {
  const dir = tempDir();
  const installed = path.join(dir, "frus.exe");
  fs.writeFileSync(installed, "version 1");
  let calls = 0;
  const watch = watchProgram(installed, () => calls++, 50);
  await new Promise((resolve) => setTimeout(resolve, 150));
  fs.writeFileSync(installed, "version 2, longer");
  await new Promise((resolve) => setTimeout(resolve, 500));
  watch.dispose();
  assert.equal(calls, 1);
});

// What it is all for: the installed program can be replaced while its copy runs, which Windows
// refuses for a program that is running.
test("the installed program can be replaced while its copy runs", { skip: process.platform !== "win32" }, async () => {
  const dir = tempDir();
  const installed = path.join(dir, "frus.exe");
  fs.copyFileSync(process.execPath, installed);
  const copy = runnableCopy(installed, path.join(dir, "copies"));
  const running = spawn(copy, ["-e", "setTimeout(() => {}, 10000)"], { stdio: "ignore" });
  try {
    await new Promise((resolve) => setTimeout(resolve, 500));
    const next = path.join(dir, "frus-next.exe");
    fs.writeFileSync(next, "a new install");
    fs.renameSync(next, installed); // What `cargo install` does; "access denied" if in use.
    assert.equal(fs.readFileSync(installed, "utf8"), "a new install");
  } finally {
    running.kill();
  }
});
