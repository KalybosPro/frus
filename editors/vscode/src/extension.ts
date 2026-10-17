// The frus extension: a device picker in the status bar, F5 to run on it with the
// application's log in the Debug Console, and a reload on save.

import * as path from "path";
import * as vscode from "vscode";

import { FrusDebugAdapter } from "./adapter";
import { Devices } from "./daemon";
import { frusProgram } from "./frus";

const SELECTED = "frus.device";

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
  frusProgram().then(
    (frus) => {
      devices.start(frus);
      refresh();
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

    // F5: a configuration with the selected device and the application's directory.
    vscode.debug.registerDebugConfigurationProvider("frus", {
      async resolveDebugConfiguration(folder, config) {
        if (!config.type) {
          config = { type: "frus", request: "launch", name: "frus" };
        }
        config.cwd ??= appDirectory(folder);
        config.deviceId ||= selected();
        try {
          config.frus = await frusProgram();
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

    // Saving a Rust file reloads the running application.
    vscode.workspace.onDidSaveTextDocument((document) => {
      const reloadOnSave = vscode.workspace.getConfiguration("frus").get<boolean>("reloadOnSave");
      if (reloadOnSave && document.languageId === "rust") customRequest("hotReload");
    }),
  );
}

export function deactivate(): void {}

/** A request to every running frus session. */
function customRequest(command: string): void {
  const sessions = [vscode.debug.activeDebugSession].filter(
    (session): session is vscode.DebugSession => session?.type === "frus",
  );
  for (const session of sessions) session.customRequest(command);
}

/** Where the application is: the folder of the open Rust file's nearest Cargo.toml inside the
 *  workspace, or the workspace folder. */
function appDirectory(folder: vscode.WorkspaceFolder | undefined): string | undefined {
  const root = folder?.uri.fsPath ?? vscode.workspace.workspaceFolders?.[0]?.uri.fsPath;
  const file = vscode.window.activeTextEditor?.document.uri.fsPath;
  if (!root || !file || !file.startsWith(root)) return root;
  let dir = path.dirname(file);
  const fs = require("fs") as typeof import("fs");
  while (dir.startsWith(root)) {
    if (fs.existsSync(path.join(dir, "Cargo.toml"))) return dir;
    const parent = path.dirname(dir);
    if (parent === dir) break;
    dir = parent;
  }
  return root;
}
