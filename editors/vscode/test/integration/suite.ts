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
    for (const command of ["frus.selectDevice", "frus.reload", "frus.restart"]) {
      if (!commands.includes(command)) throw new Error(`no command ${command}`);
    }
    step("its commands are registered");

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

    await vscode.debug.stopDebugging();
    await until("the end of the session", () => seen.some((m) => m.event === "terminated"), 60_000);
    step("stopping the session ends it");
    report();
  } catch (error) {
    report(error);
    throw error;
  }
}
