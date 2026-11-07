// The debug adapter behind F5: it speaks the Debug Adapter Protocol to VS Code and drives
// `frus run --machine` underneath. It has no dependency on the `vscode` module, so its tests run
// with plain Node.
//
// What it maps:
//   launch                → `frus run --machine -d <device>` in the application's directory
//   app.log               → an output event: the Debug Console, stdout or stderr
//   app.progress, stderr  → output in the console category (the build, the tool's messages)
//   restart (toolbar)     → app.restart with fullRestart: true
//   `hotReload` (custom)  → app.restart with fullRestart: false
//   `key` (custom)        → app.key: any other key of the run's, `{ key: "p" }`
//   disconnect/terminate  → app.stop, then the process is ended if it lingers
//   app.stop              → its error, if any, in the console
//   exit                  → the exit code (and, on a failure, the last error), then terminated

import { ChildProcess, spawn } from "child_process";

import { readLines, send } from "./lines";

/** What `launch` is given (the launch configuration, resolved). */
export interface LaunchArguments {
  cwd: string;
  deviceId?: string;
  release?: boolean;
  /** True hot reload (desktop, web, Android): each saved change patched into the running
   *  application. Unset: on for a debug build, passed as `--hot` so that every frus has it (frus
   *  0.5.0's own default is off); true: `--hot`, a release build too; false: `--no-hot`;
   *  `"frus"`: nothing passed, frus decides. */
  hot?: boolean | "frus";
  args?: string[];
  /** The `frus` program. */
  frus: string;
  /** Whether that program takes `--no-hot` (frus 0.5.0 does not, and its default is off).
   *  Unset: it does. */
  knowsNoHot?: boolean;
}

/** The arguments of the `frus run --machine` a launch starts. */
export function runArguments(args: Omit<LaunchArguments, "cwd" | "frus">): string[] {
  const command = ["run", "--machine"];
  if (args.deviceId) command.push("-d", args.deviceId);
  if (args.release) command.push("--release");
  const hot = args.hot ?? !args.release;
  if (hot === true) command.push("--hot");
  if (args.hot === false && args.knowsNoHot !== false) command.push("--no-hot");
  if (args.args?.length) command.push("--", ...args.args);
  return command;
}

/**
 * How a run ended, for the console's last line: every end is shown, with its exit code, and an
 * end that is not a success with the last error seen, so that a run never stops in silence.
 */
export class RunEnding {
  private lastError: string | undefined;
  private stopSaid = false;

  /** Something shown as an error, or `app.stop` (with its error, if any). */
  saw(text: string | undefined | null, error: boolean, stopped = false): void {
    if (stopped) this.stopSaid = true;
    if (error && text && text.trim()) this.lastError = text.trim();
  }

  /** A line of the tool's standard error: one that reads as an error is remembered. */
  sawStderr(line: string): void {
    const text = line.trim();
    if (/^(error(\[\w+\])?:|Error:|thread '.*' panicked)|\bpanicked at\b/.test(text)) this.lastError = text;
  }

  /** The console's last line for an exit with `code` (null: ended by `signal`). */
  message(code: number | null, signal?: string | null): { text: string; error: boolean } {
    if (code === 0) {
      const how = this.stopSaid ? "" : ", without saying it was stopping";
      return { text: `frus run ended (exit code 0${how}).`, error: false };
    }
    const how = code === null ? `ended by ${signal ?? "a signal"}` : `exit code ${code}`;
    const why = this.lastError ? ` The last error: ${this.lastError}` : " It said no error: the lines above are all it printed.";
    return { text: `frus run failed (${how}).${why}`, error: true };
  }
}

type Message = { seq?: number; type: string; [key: string]: any };
type Listener = (message: Message) => void;

/** How the adapter starts `frus`: replaced in tests. */
export type Spawner = (program: string, args: string[], cwd: string) => ChildProcess;

const defaultSpawner: Spawner = (program, args, cwd) =>
  spawn(program, args, { cwd, stdio: ["pipe", "pipe", "pipe"], windowsHide: true });

export class FrusDebugAdapter {
  private listeners: Listener[] = [];
  private seq = 1;
  private run: ChildProcess | undefined;
  private nextCommandId = 1;
  private ended = false;
  private ending = new RunEnding();

  constructor(private readonly spawner: Spawner = defaultSpawner) {}

  /** What VS Code subscribes to: the adapter's messages. */
  readonly onDidSendMessage = (listener: Listener) => {
    this.listeners.push(listener);
    return {
      dispose: () => {
        this.listeners = this.listeners.filter((l) => l !== listener);
      },
    };
  };

  /** A request from VS Code. */
  handleMessage(message: Message): void {
    if (message.type !== "request") return;
    const respond = (body: object = {}, success = true, error?: string) =>
      this.emit({
        type: "response",
        request_seq: message.seq,
        command: message.command,
        success,
        message: error,
        body,
      });
    switch (message.command) {
      case "initialize":
        respond({
          supportsConfigurationDoneRequest: true,
          supportsRestartRequest: true,
          supportsTerminateRequest: true,
        });
        this.event("initialized");
        break;
      case "launch":
        try {
          this.launch(message.arguments as LaunchArguments);
          respond();
        } catch (error) {
          respond({}, false, String(error));
        }
        break;
      case "configurationDone":
        respond();
        break;
      case "threads":
        respond({ threads: [{ id: 1, name: "main" }] });
        break;
      case "restart":
        this.restart(true);
        respond();
        break;
      case "hotReload":
        this.restart(false);
        respond();
        break;
      case "hotRestart":
        this.restart(true);
        respond();
        break;
      case "key":
        this.key(String((message.arguments as { key?: string } | undefined)?.key ?? ""));
        respond();
        break;
      case "disconnect":
      case "terminate":
        this.stop();
        respond();
        break;
      default:
        respond();
    }
  }

  dispose(): void {
    this.stop();
  }

  private launch(args: LaunchArguments): void {
    const command = runArguments(args);
    this.output(`frus ${command.join(" ")}\n`, "console");
    const run = this.spawner(args.frus, command, args.cwd);
    this.run = run;
    readLines(
      run.stdout!,
      (message) => this.fromFrus(message),
      (line) => this.output(line + "\n", "console"),
    );
    // The build's own output (cargo, cargo-apk) and the tool's diagnostics.
    readLines(run.stderr!, () => {}, (line) => {
      this.ending.sawStderr(line);
      this.output(line + "\n", "console");
    });
    run.on("error", (error) => {
      this.output(`Could not start frus: ${error.message}\n`, "stderr");
      this.end();
    });
    // Once its output is all read: every end in the console, with its exit code.
    run.on("close", (code: number | null, signal: string | null) => {
      if (this.ended) return;
      const said = this.ending.message(code, signal);
      this.output(said.text + "\n", said.error ? "stderr" : "console");
      this.end();
    });
  }

  /** An event of `frus run --machine`. */
  private fromFrus(message: any): void {
    const params = message.params ?? {};
    switch (message.event) {
      case "app.log":
        this.ending.saw(params.log, Boolean(params.error));
        this.output(params.log + "\n", params.error ? "stderr" : "stdout");
        break;
      case "app.progress":
        this.ending.saw(params.message, Boolean(params.error));
        this.output(params.message + "\n", params.error ? "stderr" : "console");
        break;
      case "app.start":
        this.output(`Running on ${params.deviceName}…\n`, "console");
        break;
      case "app.started":
        this.output("The application is running. Saving a Rust file reloads it.\n", "console");
        break;
      case "app.restarted": {
        const what = params.fullRestart ? "Restarted" : "Reloaded";
        this.output(`${what} in ${Number(params.seconds).toFixed(1)} s.\n`, "console");
        break;
      }
      case "app.stop": {
        this.ending.saw(params.error, true, true);
        if (params.error) this.output(params.error + "\n", "stderr");
        // frus ends right after; its exit is reported then. If it lingers, the session ends.
        const run = this.run;
        setTimeout(() => {
          if (run && run.exitCode === null) this.end();
        }, 5000).unref?.();
        break;
      }
    }
  }

  private restart(full: boolean): void {
    if (!this.run || this.ended) return;
    this.output(full ? "Restarting…\n" : "Reloading…\n", "console");
    send(this.run, {
      id: this.nextCommandId++,
      method: "app.restart",
      params: { fullRestart: full },
    });
  }

  /** A key of `frus run`'s: what it does is reported in the log, as in the terminal. */
  private key(key: string): void {
    if (!this.run || this.ended || key.length !== 1) return;
    send(this.run, { id: this.nextCommandId++, method: "app.key", params: { key } });
  }

  private stop(): void {
    const run = this.run;
    if (!run || this.ended) return;
    send(run, { id: this.nextCommandId++, method: "app.stop" });
    // It stops the application and ends on its own; if it does not, it is ended.
    setTimeout(() => {
      if (run.exitCode === null) run.kill();
    }, 5000).unref?.();
  }

  private end(): void {
    if (this.ended) return;
    this.ended = true;
    this.event("terminated");
  }

  private output(text: string, category: "console" | "stdout" | "stderr"): void {
    this.event("output", { category, output: text });
  }

  private event(event: string, body: object = {}): void {
    this.emit({ type: "event", event, body });
  }

  private emit(message: Message): void {
    const full = { seq: this.seq++, ...message };
    for (const listener of this.listeners) listener(full);
  }
}
