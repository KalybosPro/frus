// A new frus application: the name it may have, and the `frus create` that makes it. Free of
// `vscode`, so the rules are tested on their own.

import * as fs from "fs";
import * as path from "path";

/** The organisation a new application's Android package starts with, unless told. */
export const DEFAULT_ORG = "com.example";

/**
 * Why `name` cannot be a new application's name, or `undefined` when it can: what `frus create`
 * accepts (lowercase letters, digits, `_` and `-`), starting with a letter, and not a folder that
 * is already there with something in it.
 */
export function nameProblem(name: string, parent: string | undefined): string | undefined {
  if (!name) {
    return "A name, please: it is the folder's and the package's.";
  }
  if (!/^[a-z]/.test(name)) {
    return "Start with a lowercase letter.";
  }
  if (!/^[a-z][a-z0-9_-]*$/.test(name)) {
    return "Lowercase letters, digits, `_` and `-` only, as a Rust package name.";
  }
  if (parent) {
    const target = path.join(parent, name);
    try {
      if (fs.readdirSync(target).length > 0) {
        return `${target} is already there, and not empty.`;
      }
    } catch {
      // Not there: it is made.
    }
  }
  return undefined;
}

/**
 * Why `org` cannot start an Android package name, or `undefined`: dot-separated parts, each a
 * letter then letters, digits or `_`.
 */
export function orgProblem(org: string): string | undefined {
  if (!/^[A-Za-z][A-Za-z0-9_]*(\.[A-Za-z][A-Za-z0-9_]*)+$/.test(org)) {
    return "A reverse domain such as com.acme: parts separated by dots, each starting with a letter.";
  }
  return undefined;
}

/** The arguments of the `frus create` that makes `name` in `parent`. */
export function createArgs(parent: string, name: string, org: string): string[] {
  const args = ["create", path.join(parent, name)];
  if (org && org !== DEFAULT_ORG) {
    args.push("--org", org);
  }
  return args;
}
