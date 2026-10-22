// Whether the installed `frus` is behind the one on crates.io. Free of `vscode`, so it is tested
// on its own.

/** The version in `frus --version`'s answer (`frus 0.4.0`), or `undefined`. */
export function versionOf(answer: string): string | undefined {
  return answer.match(/(\d+)\.(\d+)\.(\d+)/)?.[0];
}

/** Whether version `a` is older than `b`, both `major.minor.patch`. */
export function isOlder(a: string, b: string): boolean {
  const parts = (v: string) => v.split(".").map((n) => parseInt(n, 10) || 0);
  const [x, y] = [parts(a), parts(b)];
  for (let i = 0; i < 3; i++) {
    if (x[i] !== y[i]) return x[i] < y[i];
  }
  return false;
}

/**
 * The `cargo install` that brings `frus` to the newest version. `--force` replaces whatever is
 * installed now, however it got there: without it, a `frus` installed from a checkout (a path)
 * makes cargo refuse, the binary belonging to another source.
 */
export const UPDATE_ARGS = ["install", "frus-cli", "--locked", "--force"];
