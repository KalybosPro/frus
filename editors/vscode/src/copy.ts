// The `frus` program the extension runs is a copy of the installed one, so that the installed
// one is never in use: Windows refuses to replace a running program, and a daemon kept open for
// the window would make every `cargo install frus-cli` fail with "access denied".

import * as fs from "fs";
import * as path from "path";

/** The file `program` runs: itself when it is a path, else the first match on the PATH. */
export function resolveProgram(
  program: string,
  env: NodeJS.ProcessEnv = process.env,
  platform: NodeJS.Platform = process.platform,
): string | undefined {
  const isFile = (file: string) => {
    try {
      return fs.statSync(file).isFile();
    } catch {
      return false;
    }
  };
  const extensions =
    platform === "win32"
      ? ["", ...(env.PATHEXT ?? ".EXE;.CMD;.BAT").split(";").filter(Boolean).map((e) => e.toLowerCase())]
      : [""];
  if (program.includes("/") || program.includes("\\")) {
    return extensions.map((ext) => program + ext).find(isFile);
  }
  const separator = platform === "win32" ? ";" : ":";
  for (const dir of (env.PATH ?? env.Path ?? "").split(separator).filter(Boolean)) {
    const found = extensions.map((ext) => path.join(dir, program + ext)).find(isFile);
    if (found) return found;
  }
  return undefined;
}

/** A copy of `program` in `dir` to run instead of it, made if it is not there yet. Named after
 *  the original's size and time, so a new install gets a new copy; older copies are removed
 *  when nothing runs them any more. */
export function runnableCopy(program: string, dir: string): string {
  const stat = fs.statSync(program);
  const extension = path.extname(program);
  const name = `frus-${Math.trunc(stat.mtimeMs)}-${stat.size}${extension}`;
  const copy = path.join(dir, name);
  fs.mkdirSync(dir, { recursive: true });
  if (!fs.existsSync(copy)) {
    // Written under another name first: a copy cut short must never be taken for a whole one.
    const partial = `${copy}.${process.pid}.part`;
    fs.copyFileSync(program, partial);
    fs.renameSync(partial, copy);
  }
  for (const other of fs.readdirSync(dir)) {
    if (other !== name && other.startsWith("frus-")) {
      try {
        fs.unlinkSync(path.join(dir, other));
      } catch {
        // Still running, in another window: it goes next time.
      }
    }
  }
  return copy;
}

/** Calls `changed` when `program` is replaced, once it has stayed the same for a moment (an
 *  install writes it, then may touch it again). */
export function watchProgram(program: string, changed: () => void, interval = 2000): { dispose(): void } {
  let settle: NodeJS.Timeout | undefined;
  const listener = (now: fs.Stats, before: fs.Stats) => {
    if (now.mtimeMs === before.mtimeMs && now.size === before.size) return;
    if (now.size === 0 && now.mtimeMs === 0) return; // Gone for a moment, mid-replacement.
    clearTimeout(settle);
    settle = setTimeout(changed, interval);
  };
  fs.watchFile(program, { interval }, listener);
  return {
    dispose: () => {
      clearTimeout(settle);
      fs.unwatchFile(program, listener);
    },
  };
}
