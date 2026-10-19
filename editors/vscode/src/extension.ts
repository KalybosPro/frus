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
  // What the daemon says on its way (installing, creating, starting), and what it wants seen.
  const output = vscode.window.createOutputChannel("frus");
  context.subscriptions.push(
    output,
    devices.onMessage((said) => {
      output.appendLine(said.message);
      if (said.kind === "show") {
        const show =
          said.level === "error"
            ? vscode.window.showErrorMessage
            : said.level === "warning"
              ? vscode.window.showWarningMessage
              : vscode.window.showInformationMessage;
        show(`frus: ${said.message}`);
      }
    }),
  );
  // An emulator started from the picker is selected once it is up: by its name, which a running
  // emulator takes from its AVD.
  let awaitedEmulator: string | undefined;
  const launch = async (id: string, name: string) => {
    awaitedEmulator = name;
    try {
      await devices.launchEmulator(id);
      vscode.window.setStatusBarMessage(`frus: starting ${name}…`, 15000);
    } catch (error: any) {
      awaitedEmulator = undefined;
      vscode.window.showErrorMessage(`frus: ${error.message}`);
    }
  };
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
  devices.onDidChange(() => {
    if (awaitedEmulator) {
      const up = devices.list().find((d) => d.emulator && d.available && d.name === awaitedEmulator && d.id.startsWith("emulator-"));
      if (up) {
        awaitedEmulator = undefined;
        void context.workspaceState.update(SELECTED, up.id).then(refresh);
        vscode.window.showInformationMessage(`frus: ${up.name} is running and selected.`);
      }
    }
    refresh();
  });

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
      const picked = await vscode.window.showQuickPick(pickerItems(devices, context), {
        placeHolder: "Where should frus run the application?",
      });
      if (!picked) return;
      if (picked.action === "device") {
        await context.workspaceState.update(SELECTED, picked.id);
        refresh();
      } else if (picked.action === "start") {
        await launch(picked.id!, picked.name!);
      } else if (picked.action === "create") {
        await createEmulator(devices, (name) => (awaitedEmulator = name));
      }
    }),
    // The emulators on their own, as commands: the picker's entries do the same.
    vscode.commands.registerCommand("frus.launchEmulator", async (emulatorId?: string) => {
      const emulators = await devices.emulators().catch(() => []);
      let chosen = emulators.find((e) => e.id === emulatorId);
      if (!chosen) {
        const picked = await vscode.window.showQuickPick(
          emulators.map((e) => ({ label: e.name, description: `${e.platformType} emulator`, emulator: e })),
          { placeHolder: emulators.length ? "Which emulator should start?" : "No emulator: create one first" },
        );
        chosen = picked?.emulator;
      }
      if (chosen) await launch(chosen.id, chosen.name);
    }),
    // `frus doctor -v` in a terminal of its own: what this machine has, and what it lacks.
    vscode.commands.registerCommand("frus.runDoctor", async () => {
      try {
        const frus = await runnable();
        vscode.window
          .createTerminal({ name: "frus doctor", shellPath: frus, shellArgs: ["doctor", "-v"] })
          .show();
      } catch (error: any) {
        vscode.window.showErrorMessage(`frus: ${error.message}`);
      }
    }),
    vscode.commands.registerCommand("frus.createEmulator", () =>
      createEmulator(devices, (name) => (awaitedEmulator = name)),
    ),
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

/** An entry of the device picker: a device to select, an emulator to start, or the one that
 *  makes an emulator. */
interface PickerItem extends vscode.QuickPickItem {
  action?: "device" | "start" | "create";
  id?: string;
  name?: string;
}

/** The picker: the devices that can run the application now, then the emulators that can be
 *  started, then the one that makes a new emulator. A device whose platform the application does
 *  not build for says why. */
async function pickerItems(devices: Devices, context: vscode.ExtensionContext): Promise<PickerItem[]> {
  const root = context.workspaceState.get<string>(APP) ?? vscode.workspace.workspaceFolders?.[0]?.uri.fsPath;
  const support = root && isApplication(root) ? await devices.supportedPlatforms(root).catch(() => undefined) : undefined;
  const running = devices.list().filter((d) => !(d.emulator && !d.id.startsWith("emulator-") && d.platform === "android"));
  const items: PickerItem[] = running.map((device) => {
    const unsupported = support?.platformTypes[device.platform];
    const why = unsupported && !unsupported.isSupported ? unsupported.reasons?.[0]?.reasonText : undefined;
    return {
      label: device.name,
      description: `${device.id} • ${device.detail}`,
      detail: !device.available
        ? `$(warning) ${device.unavailableReason}`
        : why
          ? `$(warning) not for this application: ${why}`
          : undefined,
      action: "device",
      id: device.id,
    };
  });
  const emulators = await devices.emulators().catch(() => []);
  const up = new Set(devices.list().filter((d) => d.id.startsWith("emulator-")).map((d) => d.name));
  const startable = emulators.filter((e) => !up.has(e.name));
  items.push({ label: "Emulators", kind: vscode.QuickPickItemKind.Separator });
  for (const emulator of startable) {
    items.push({
      label: `$(play) Start ${emulator.name}`,
      description: `${emulator.platformType} emulator`,
      action: "start",
      id: emulator.id,
      name: emulator.name,
    });
  }
  items.push({
    label: "$(add) Create an Android emulator",
    description: "installs what it needs",
    action: "create",
  });
  return items;
}

/** Makes an emulator with the name asked for, and offers to start it. */
async function createEmulator(devices: Devices, awaiting: (name: string) => void): Promise<void> {
  const name = await vscode.window.showInputBox({
    prompt: "A name for the new Android emulator",
    placeHolder: "frus_emulator",
  });
  if (name === undefined) return;
  const result = await vscode.window.withProgress(
    {
      location: vscode.ProgressLocation.Notification,
      title: "frus: creating an Android emulator (the first one may download a system image)…",
    },
    () => devices.createEmulator(name.trim() || undefined).catch((error: Error) => ({ success: false, emulatorName: null, error: error.message })),
  );
  if (!result.success || !result.emulatorName) {
    vscode.window.showErrorMessage(`frus: the emulator was not created: ${result.error}`);
    return;
  }
  const start = await vscode.window.showInformationMessage(`frus: ${result.emulatorName} was created.`, "Start it");
  if (start) {
    const shown = result.emulatorName.replace(/_/g, " ");
    awaiting(shown);
    await devices.launchEmulator(result.emulatorName).catch((error: Error) =>
      vscode.window.showErrorMessage(`frus: ${error.message}`),
    );
  }
}

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
