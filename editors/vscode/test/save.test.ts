// When a save reloads the running application.

import assert from "node:assert/strict";
import { test } from "node:test";

import { savesReload } from "../src/save";

test("a save reloads as frus.reloadOnSave says", () => {
  // [mode, by hand, with changes] → reloads
  const cases: Array<[Parameters<typeof savesReload>[0], boolean, boolean, boolean]> = [
    ["never", true, true, false],
    ["manual", true, false, true],
    ["manual", false, true, false],
    ["manualIfDirty", true, true, true],
    ["manualIfDirty", true, false, false],
    ["manualIfDirty", false, true, false],
    ["all", false, false, true],
  ];
  for (const [mode, manual, dirty, reloads] of cases) {
    assert.equal(savesReload(mode, manual, dirty), reloads, `${mode} ${manual} ${dirty}`);
  }
});
