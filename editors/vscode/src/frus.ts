// The `frus` program: the one the settings name, or the one on the PATH — installed with
// `cargo install frus-cli` when there is none, rather than asking the developer to.

import { execFile } from "child_process";
import * as vscode from "vscode";

let ready: Promise<string> | undefined;

/** The `frus` program to run, installed first if it has to be. */
export function frusProgram(): Promise<string> {
  ready ??= locate().catch((error) => {
    ready = undefined;
    throw error;
  });
  return ready;
}

async function locate(): Promise<string> {
  const configured = vscode.workspace.getConfiguration("frus").get<string>("path")?.trim();
  const program = configured || "frus";
  if (await answers(program)) {
    if (!(await speaksToEditors(program))) {
      throw new Error(
        `${program} is older than the editor protocol (\`frus daemon\`, \`frus run --machine\`): ` +
          "update it with `cargo install frus-cli --force`, or set frus.path to a newer one.",
      );
    }
    return program;
  }
  if (configured) {
    throw new Error(`frus.path names ${configured}, which does not run.`);
  }
  await vscode.window.withProgress(
    {
      location: vscode.ProgressLocation.Notification,
      title: "Installing frus (cargo install frus-cli, a few minutes the first time)…",
    },
    () => run("cargo", ["install", "frus-cli", "--locked"]),
  );
  if (!(await answers(program))) {
    throw new Error("frus-cli was installed, but `frus` is not on the PATH (is ~/.cargo/bin there?).");
  }
  return program;
}

/** Whether `program` has the editor protocol: its `run` takes `--machine`. */
function speaksToEditors(program: string): Promise<boolean> {
  return new Promise((resolve) =>
    execFile(program, ["help", "run"], { windowsHide: true }, (error, out) =>
      resolve(!error && out.includes("--machine")),
    ),
  );
}

function answers(program: string): Promise<boolean> {
  return new Promise((resolve) =>
    execFile(program, ["--version"], { windowsHide: true }, (error) => resolve(!error)),
  );
}

function run(program: string, args: string[]): Promise<void> {
  return new Promise((resolve, reject) =>
    execFile(program, args, { windowsHide: true, maxBuffer: 64 * 1024 * 1024 }, (error, _out, err) =>
      error ? reject(new Error(`${program} ${args.join(" ")} failed:\n${err}`)) : resolve(),
    ),
  );
}
