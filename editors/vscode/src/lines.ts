// JSON lines in and out of a `frus` process: one object a line, nothing else on standard output.

import { ChildProcess } from "child_process";

/** Calls `onMessage` with each JSON object `stream` writes, a line at a time. Lines that are not
 *  JSON are given to `onOther`. */
export function readLines(
  stream: NodeJS.ReadableStream,
  onMessage: (message: any) => void,
  onOther: (line: string) => void = () => {},
): void {
  let pending = "";
  stream.setEncoding?.("utf8");
  stream.on("data", (chunk: string | Buffer) => {
    pending += chunk.toString();
    let at: number;
    while ((at = pending.indexOf("\n")) >= 0) {
      const line = pending.slice(0, at).replace(/\r$/, "");
      pending = pending.slice(at + 1);
      if (!line.trim()) continue;
      let message: unknown;
      try {
        message = JSON.parse(line);
      } catch {
        onOther(line);
        continue;
      }
      onMessage(message);
    }
  });
}

/** Writes one command to a `frus` process. */
export function send(process: ChildProcess, command: object): void {
  process.stdin?.write(JSON.stringify(command) + "\n");
}
