// Runs the integration suite inside a VS Code of its own (downloaded once), never the developer's.
//
//   FRUS_E2E_FRUS=/path/to/frus FRUS_E2E_APP=/path/to/an/app node out/test/integration/run.js
//
// The application is opened as the workspace, with `frus.path` set to FRUS_E2E_FRUS in its
// `.vscode/settings.json`.

import * as fs from "fs";
import * as path from "path";

import { runTests } from "@vscode/test-electron";

async function main(): Promise<void> {
  const frus = process.env.FRUS_E2E_FRUS;
  const app = process.env.FRUS_E2E_APP;
  if (!frus || !app) {
    console.log("FRUS_E2E_FRUS and FRUS_E2E_APP are not set: nothing to run.");
    return;
  }
  // An instance of its own: its own profile and extensions, whatever VS Code is open already.
  const isolated = fs.mkdtempSync(path.join(require("os").tmpdir(), "frus-vscode-"));
  // And a `frus` of its own, which the suite replaces as an install would.
  const installed = path.join(isolated, "bin", path.basename(frus));
  fs.mkdirSync(path.dirname(installed), { recursive: true });
  fs.copyFileSync(frus, installed);
  const settings = path.join(app, ".vscode", "settings.json");
  fs.mkdirSync(path.dirname(settings), { recursive: true });
  fs.writeFileSync(settings, JSON.stringify({ "frus.path": installed, "frus.reloadOnSave": false }, null, 2));

  const extensionDevelopmentPath = path.resolve(__dirname, "../../..");
  const result = path.join(app, ".vscode", "frus-integration.json");
  fs.rmSync(result, { force: true });
  await runTests({
    extensionDevelopmentPath,
    extensionTestsPath: path.resolve(__dirname, "suite"),
    launchArgs: [
      app,
      "--disable-extensions",
      "--skip-welcome",
      "--skip-release-notes",
      `--user-data-dir=${path.join(isolated, "data")}`,
      `--extensions-dir=${path.join(isolated, "extensions")}`,
    ],
    extensionTestsEnv: { ...process.env, FRUS_INTEGRATION_RESULT: result, FRUS_INTEGRATION_FRUS: installed } as Record<string, string>,
    // An installed VS Code (FRUS_VSCODE), or one downloaded into FRUS_VSCODE_CACHE.
    vscodeExecutablePath: process.env.FRUS_VSCODE,
    cachePath: process.env.FRUS_VSCODE_CACHE,
    timeout: 120_000,
  });
  fs.rmSync(isolated, { recursive: true, force: true });
  console.log(fs.readFileSync(result, "utf8"));
}

main().catch((error) => {
  console.error(error);
  process.exit(1);
});
