import * as assert from "assert";
import { test } from "node:test";

import { isOlder, UPDATE_ARGS, versionOf } from "../src/update";

test("the version is read from frus --version", () => {
  assert.strictEqual(versionOf("frus 0.4.0"), "0.4.0");
  assert.strictEqual(versionOf("frus 0.10.2\n"), "0.10.2");
  assert.strictEqual(versionOf("command not found"), undefined);
});

test("versions compare part by part, not as text", () => {
  assert.ok(isOlder("0.3.0", "0.4.0"));
  assert.ok(isOlder("0.9.0", "0.10.0"), "10 is more than 9");
  assert.ok(isOlder("0.4.0", "1.0.0"));
  assert.ok(!isOlder("0.4.0", "0.4.0"), "the same is not older");
  assert.ok(!isOlder("0.4.1", "0.4.0"));
});

test("the update replaces an install from anywhere", () => {
  assert.ok(UPDATE_ARGS.includes("--force"));
});
