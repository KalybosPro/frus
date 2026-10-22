import * as assert from "assert";
import * as fs from "fs";
import * as os from "os";
import * as path from "path";
import { test } from "node:test";

import { createArgs, DEFAULT_ORG, nameProblem, orgProblem } from "../src/project";

test("a name is what frus create accepts", () => {
  assert.strictEqual(nameProblem("my_app", undefined), undefined);
  assert.strictEqual(nameProblem("shop-2", undefined), undefined);
  assert.ok(nameProblem("", undefined));
  assert.ok(nameProblem("MyApp", undefined), "uppercase");
  assert.ok(nameProblem("2app", undefined), "a digit first");
  assert.ok(nameProblem("my app", undefined), "a space");
  assert.ok(nameProblem("café", undefined), "an accent");
});

test("a folder already there with files in it is refused, an empty one is not", () => {
  const parent = fs.mkdtempSync(path.join(os.tmpdir(), "frus-project-"));
  try {
    fs.mkdirSync(path.join(parent, "empty"));
    fs.mkdirSync(path.join(parent, "full"));
    fs.writeFileSync(path.join(parent, "full", "Cargo.toml"), "");
    assert.strictEqual(nameProblem("empty", parent), undefined);
    assert.strictEqual(nameProblem("fresh", parent), undefined);
    assert.match(nameProblem("full", parent) ?? "", /already there/);
  } finally {
    fs.rmSync(parent, { recursive: true, force: true });
  }
});

test("an organisation is a reverse domain", () => {
  assert.strictEqual(orgProblem("com.acme"), undefined);
  assert.strictEqual(orgProblem("io.github.kalybos_pro"), undefined);
  assert.ok(orgProblem("acme"), "one part");
  assert.ok(orgProblem("com.1acme"), "a part starting with a digit");
  assert.ok(orgProblem("com..acme"));
});

test("frus create is given the folder, and the organisation only when it is not the default", () => {
  const parent = path.join("work", "apps");
  assert.deepStrictEqual(createArgs(parent, "shop", DEFAULT_ORG), ["create", path.join(parent, "shop")]);
  assert.deepStrictEqual(createArgs(parent, "shop", "com.acme"), [
    "create",
    path.join(parent, "shop"),
    "--org",
    "com.acme",
  ]);
});
