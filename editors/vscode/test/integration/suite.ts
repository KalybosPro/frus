// What runs inside the test VS Code: the extension, used the way a developer uses it, observed
// through VS Code's own API (a debug adapter tracker sees every message the adapter sends).

import * as fs from "fs";
import * as vscode from "vscode";

export async function run(): Promise<void> {
  const steps: string[] = [];
  const step = (text: string) => steps.push(text);
  const seen: any[] = [];
  const report = (error?: unknown) =>
    fs.writeFileSync(
      process.env.FRUS_INTEGRATION_RESULT!,
      JSON.stringify({ ok: !error, error: error ? String(error) : undefined, steps }, null, 2),
    );
  const until = async (what: string, test: () => boolean, ms: number) => {
    const end = Date.now() + ms;
    while (!test()) {
      if (Date.now() > end) throw new Error(`timed out waiting for ${what}`);
      await new Promise((r) => setTimeout(r, 250));
    }
  };
  const console_ = () =>
    seen
      .filter((m) => m.event === "output" && m.body?.category === "console")
      .map((m) => m.body.output)
      .join("");
  const appLines = () =>
    seen.filter((m) => m.event === "output" && ["stdout", "stderr"].includes(m.body?.category)).length;

  try {
    const extension = vscode.extensions.getExtension("kalybospro.frus");
    if (!extension) throw new Error("the extension is not loaded");
    await extension.activate();
    step("the extension activates");

    const commands = await vscode.commands.getCommands(true);
    for (const command of [
      "frus.selectDevice",
      "frus.reload",
      "frus.restart",
      "frus.createProject",
      "frus.runDoctor",
      "frus.update",
    ]) {
      if (!commands.includes(command)) throw new Error(`no command ${command}`);
    }
    step("its commands are registered");

    // frus: Create New Project, with the developer's answers given for them: the folder, the
    // name, the organisation; the new folder is caught on its way to being opened.
    const createIn = process.env.FRUS_INTEGRATION_CREATE;
    if (createIn) {
      const window = vscode.window as any;
      const commandsApi = vscode.commands as any;
      const answers = ["probe_app", "com.acme"];
      const asked: string[] = [];
      let opened: vscode.Uri | undefined;
      const saved = {
        open: window.showOpenDialog,
        input: window.showInputBox,
        execute: commandsApi.executeCommand,
      };
      window.showOpenDialog = async () => [vscode.Uri.file(createIn)];
      window.showInputBox = async (options: vscode.InputBoxOptions) => {
        const answer = answers.shift()!;
        asked.push(`${options.title}: ${answer}`);
        const problem = await options.validateInput?.(answer);
        if (problem) throw new Error(`${answer} refused: ${problem}`);
        return answer;
      };
      commandsApi.executeCommand = async (command: string, ...rest: any[]) => {
        if (command === "vscode.openFolder") {
          opened = rest[0];
          return;
        }
        return saved.execute.call(vscode.commands, command, ...rest);
      };
      try {
        await vscode.commands.executeCommand("frus.createProject");
      } finally {
        window.showOpenDialog = saved.open;
        window.showInputBox = saved.input;
        commandsApi.executeCommand = saved.execute;
      }
      const made = `${createIn}/probe_app`;
      const manifest = fs.readFileSync(`${made}/Cargo.toml`, "utf8");
      if (!manifest.includes('name = "probe_app"')) throw new Error(`no probe_app package:\n${manifest}`);
      if (!manifest.includes("com.acme.probe_app")) throw new Error(`not com.acme's:\n${manifest}`);
      const same = (a: string) => a.replace(/\\/g, "/").toLowerCase();
      if (!opened || same(opened.fsPath) !== same(made)) {
        throw new Error(`opened ${opened?.fsPath}, not ${made}`);
      }
      step(`frus.createProject asks ${asked.join(", ")}, writes the project and opens it`);
      if (process.env.FRUS_INTEGRATION_ONLY === "create") {
        report();
        return;
      }
    }

    // The device picker and the status bar, looked at: with FRUS_INTEGRATION_SHOTS set, the picker
    // is opened, a marker file asks the runner's watcher for a picture, and it is closed again.
    const shots = process.env.FRUS_INTEGRATION_SHOTS;
    if (shots) {
      await new Promise((r) => setTimeout(r, 6000)); // the daemon's first device list
      void vscode.commands.executeCommand("frus.selectDevice");
      await new Promise((r) => setTimeout(r, 4000));
      const marker = `${shots}/picker-open`;
      fs.writeFileSync(marker, "");
      await until("the picture of the picker", () => fs.existsSync(`${shots}/picker-done`), 60_000);
      step("the device picker was opened for a picture");
      if (process.env.FRUS_INTEGRATION_EMULATOR) {
        // "Start …": the command the picker's entry runs, with the emulator it names.
        await vscode.commands.executeCommand("workbench.action.closeQuickOpen");
        void vscode.commands.executeCommand("frus.launchEmulator", process.env.FRUS_INTEGRATION_AVD);
        const { execFileSync } = require("child_process") as typeof import("child_process");
        const booted = () => {
          try {
            return execFileSync(process.env.FRUS_INTEGRATION_EMULATOR!, ["devices"]).toString().includes("emulator-");
          } catch {
            return false;
          }
        };
        await until("the emulator", booted, 420_000);
        // The daemon looks every few seconds; then the extension selects it.
        await new Promise((r) => setTimeout(r, 20_000));
        fs.writeFileSync(`${shots}/selected-open`, "");
        await until("the picture of the status bar", () => fs.existsSync(`${shots}/selected-done`), 60_000);
        step("an emulator started from the picker came up");
      } else {
        await vscode.commands.executeCommand("workbench.action.closeQuickOpen");
      }
    }

    vscode.debug.registerDebugAdapterTrackerFactory("frus", {
      createDebugAdapterTracker: () => ({ onDidSendMessage: (m: any) => seen.push(m) }),
    });
    const folder = vscode.workspace.workspaceFolders![0];
    const started = await vscode.debug.startDebugging(folder, {
      type: "frus",
      request: "launch",
      name: "frus",
      deviceId: "windows",
    });
    if (!started) throw new Error("the debug session did not start");
    step("F5 starts a frus debug session");

    await until("the application", () => console_().includes("The application is running"), 900_000);
    step("the application runs");
    await until("its log", () => appLines() > 0, 60_000);
    step(`its log reaches the Debug Console (${appLines()} lines so far)`);

    // `cargo install frus-cli` while the daemon and the session run: it writes the new program
    // beside the old one and renames it over, which Windows refuses for a program in use.
    const installed = process.env.FRUS_INTEGRATION_FRUS!;
    const next = `${installed}.next`;
    fs.copyFileSync(installed, next);
    fs.renameSync(next, installed);
    step("the installed frus is replaced while the daemon and the session run");

    await new Promise((r) => setTimeout(r, 3000));
    await vscode.commands.executeCommand("frus.reload");
    await until("the reload", () => /Reloaded in [\d.]+ s/.test(console_()), 600_000);
    step(console_().match(/Reloaded in [\d.]+ s/)![0] + " (frus.reload)");

    // A key of the run's, from the command palette: the application says what it did.
    const logged = () =>
      seen
        .filter((m) => m.event === "output" && ["stdout", "stderr"].includes(m.body?.category))
        .map((m) => m.body.output)
        .join("");
    await vscode.commands.executeCommand("frus.toggleDebugPaint");
    await until("debug paint", () => logged().includes("debug paint on"), 60_000);
    step("frus.toggleDebugPaint turns debug paint on");

    await vscode.debug.stopDebugging();
    await until("the end of the session", () => seen.some((m) => m.event === "terminated"), 60_000);
    step("stopping the session ends it");
    report();
  } catch (error) {
    report(error);
    throw error;
  }
}
