// The frus extension: a device picker in the status bar, F5 to run on it with the
// application's log in the Debug Console, and a reload on save.

import * as path from "path";
import * as vscode from "vscode";

import { FrusDebugAdapter } from "./adapter";
import { resolveProgram, runnableCopy, watchProgram } from "./copy";
import { Devices } from "./daemon";
import { ReloadOnSave, savesReload } from "./save";
import { frusProgram } from "./frus";

const SELECTED = "frus.device";
const APP = "frus.application";

/** The commands that press a key of `frus run`'s, as its terminal does. */
const KEY_COMMANDS: Array<[string, string]> = [
  ["frus.toggleDebugPaint", "p"],
  ["frus.toggleInspector", "i"],
  ["frus.togglePlatform", "o"],
  ["frus.toggleBrightness", "b"],
  ["frus.togglePerformanceOverlay", "P"],
  ["frus.printWidgetTree", "w"],
  ["frus.printFocusTree", "f"],
  ["frus.printSemanticsTree", "S"],
  ["frus.screenshot", "s"],
  ["frus.detach", "d"],
];

/** The `frus.reloadOnSave` setting, read as its older true/false too. */
function reloadOnSave(): ReloadOnSave {
  const value = vscode.workspace.getConfiguration("frus").get<string | boolean>("reloadOnSave");
  if (value === true) return "all";
  if (value === false) return "never";
  return (["never", "manual", "manualIfDirty", "all"] as const).find((m) => m === value) ?? "manual";
}

export function activate(context: vscode.ExtensionContext): void {
  const devices = new Devices();
  const status = vscode.window.createStatusBarItem(vscode.StatusBarAlignment.Right, 100);
  status.command = "frus.selectDevice";
  status.tooltip = "frus: the device F5 runs the application on";
  context.subscriptions.push(status, { dispose: () => devices.dispose() });

  const selected = () => context.workspaceState.get<string>(SELECTED);
  const refresh = () => {
    const id = selected();
    const device = id ? devices.get(id) : devices.list()[0];
    status.text = `$(device-mobile) ${device ? device.name : "No device"}`;
    status.show();
  };
  devices.onDidChange(refresh);

  // The program the extension runs, daemon and F5 alike: a copy of the installed `frus`, so a
  // `cargo install frus-cli` can replace that one while the window is open.
  const copies = path.join(context.globalStorageUri.fsPath, "bin");
  let installed: string | undefined;
  const runnable = async (): Promise<string> => {
    const frus = await frusProgram();
    installed = resolveProgram(frus);
    if (!installed) return frus;
    try {
      return runnableCopy(installed, copies);
    } catch {
      return installed;
    }
  };
  runnable().then(
    (frus) => {
      devices.start(frus);
      refresh();
      if (installed) {
        // A new install: the daemon starts again from it, and F5 runs it from then on.
        const watch = watchProgram(installed, () => runnable().then((next) => devices.restart(next)));
        context.subscriptions.push(watch);
      }
    },
    (error) => vscode.window.showErrorMessage(`frus: ${error.message}`),
  );

  context.subscriptions.push(
    vscode.commands.registerCommand("frus.selectDevice", async () => {
      const items = devices.list().map((device) => ({
        label: device.name,
        description: `${device.id} • ${device.detail}`,
        detail: device.available ? undefined : `$(warning) ${device.unavailableReason}`,
        id: device.id,
      }));
      const picked = await vscode.window.showQuickPick(items, {
        placeHolder: "Where should frus run the application?",
      });
      if (picked) {
        await context.workspaceState.update(SELECTED, picked.id);
        refresh();
      }
    }),
    vscode.commands.registerCommand("frus.reload", () => customRequest("hotReload")),
    vscode.commands.registerCommand("frus.restart", () => customRequest("hotRestart")),
    ...KEY_COMMANDS.map(([command, key]) =>
      vscode.commands.registerCommand(command, () => customRequest("key", { key })),
    ),

    // F5: a configuration with the selected device and the application's directory.
    vscode.debug.registerDebugConfigurationProvider("frus", {
      async resolveDebugConfiguration(folder, config) {
        if (!config.type) {
          config = { type: "frus", request: "launch", name: "frus" };
        }
        config.cwd ??= await application(context, folder);
        if (!config.cwd) return undefined;
        config.deviceId ||= selected();
        try {
          config.frus = await runnable();
        } catch (error: any) {
          vscode.window.showErrorMessage(`frus: ${error.message}`);
          return undefined;
        }
        return config;
      },
    }),
    vscode.debug.registerDebugAdapterDescriptorFactory("frus", {
      createDebugAdapterDescriptor: () =>
        new vscode.DebugAdapterInlineImplementation(new FrusDebugAdapter() as any),
    }),

    // Saving a Rust file reloads the running application: by hand only, by default, so that
    // auto save does not reload at every pause in typing. How it was saved is known before.
    vscode.workspace.onWillSaveTextDocument((event) => {
      if (event.document.languageId !== "rust") return;
      saving.set(event.document.uri.toString(), {
        manual: event.reason === vscode.TextDocumentSaveReason.Manual,
        dirty: event.document.isDirty,
      });
    }),
    vscode.workspace.onDidSaveTextDocument((document) => {
      if (document.languageId !== "rust") return;
      const how = saving.get(document.uri.toString()) ?? { manual: true, dirty: true };
      saving.delete(document.uri.toString());
      if (savesReload(reloadOnSave(), how.manual, how.dirty)) customRequest("hotReload");
    }),
  );
}

export function deactivate(): void {}

/** How each Rust file being saved is saved, from `onWillSave` to `onDidSave`. */
const saving = new Map<string, { manual: boolean; dirty: boolean }>();

/** A request to every running frus session. */
function customRequest(command: string, args?: object): void {
  const sessions = [vscode.debug.activeDebugSession].filter(
    (session): session is vscode.DebugSession => session?.type === "frus",
  );
  for (const session of sessions) session.customRequest(command, args);
}

/** The application F5 runs: the frus application the open Rust file belongs to; else the
 *  workspace folder if it is one; else the one picked among the workspace's applications (the
 *  pick is remembered). `undefined` when there is none, or the pick was dismissed. */
async function application(
  context: vscode.ExtensionContext,
  folder: vscode.WorkspaceFolder | undefined,
): Promise<string | undefined> {
  const root = folder?.uri.fsPath ?? vscode.workspace.workspaceFolders?.[0]?.uri.fsPath;
  const file = vscode.window.activeTextEditor?.document.uri.fsPath;
  if (root && file?.startsWith(root)) {
    for (let dir = path.dirname(file); dir.startsWith(root); dir = path.dirname(dir)) {
      if (isApplication(dir)) return dir;
      if (path.dirname(dir) === dir) break;
    }
  }
  if (root && isApplication(root)) return root;

  const manifests = await vscode.workspace.findFiles(
    "**/Cargo.toml",
    "**/{target,node_modules,.git}/**",
    500,
  );
  const apps = manifests.map((uri) => path.dirname(uri.fsPath)).filter(isApplication).sort();
  if (apps.length === 0) {
    vscode.window.showErrorMessage("frus: there is no frus application in this workspace.");
    return undefined;
  }
  if (apps.length === 1) return apps[0];
  const remembered = context.workspaceState.get<string>(APP);
  if (remembered && apps.includes(remembered)) return remembered;
  const picked = await vscode.window.showQuickPick(
    apps.map((dir) => ({ label: path.basename(dir), description: root ? path.relative(root, dir) : dir, dir })),
    { placeHolder: "Which application should F5 run?" },
  );
  if (picked) await context.workspaceState.update(APP, picked.dir);
  return picked?.dir;
}

/** A directory holding a frus application: a package that depends on frus. */
function isApplication(dir: string): boolean {
  const fs = require("fs") as typeof import("fs");
  let text: string;
  try {
    text = fs.readFileSync(path.join(dir, "Cargo.toml"), "utf8");
  } catch {
    return false;
  }
  return /^\s*\[package\]/m.test(text) && /^\s*frus\s*(=|\.)/m.test(text);
}
