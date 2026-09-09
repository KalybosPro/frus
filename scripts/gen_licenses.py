#!/usr/bin/env python3
"""Generate an application's licence notices from its **actual** dependency graph.

    python scripts/gen_licenses.py -p frus-demo -o crates/frus-demo/assets/licenses.txt

Every application distributed anywhere has to show the licences of what it links, and
the one failure mode that matters is a list that does not match what was actually
linked. A hand-written list drifts the first time somebody adds a dependency; a runtime
registry that packages add themselves to — which is what the reference does — has no
equivalent in Rust, where a crate cannot run code before `main`.

So the list is read from cargo, which is the only thing that knows the answer:

  * `cargo tree -p <package> -e no-dev --target all` for **which** packages are linked.
    `no-dev` because a test harness is not shipped; `--target all` because one generated
    file has to serve every platform the application is built for, and a notice for
    something a given build did not link is harmless where a missing one is not.
  * `cargo metadata` for each package's manifest directory and its declared SPDX
    expression.
  * the licence files the package actually ships, read from the manifest's own
    directory: `LICENSE*`, `LICENCE*`, `COPYING*`, `UNLICENSE*`, `NOTICE*`.

Packages sharing a **byte-identical** text are grouped into one notice, which is what
keeps 431 packages down to a couple of hundred entries. Texts that differ are kept
apart, even when they are the same licence with a different appendix — the tools that
group by licence *identity* rather than by text save about 340 KB here, and this one
does not, because "close enough to Apache-2.0" is the first step towards a list that
does not say what the application ships.

A package that declares a licence and ships no file for it is not dropped: those are
collected per declared expression into one entry that says so, with the repository to
look at. That is the honest report, and it is also the list to take to those projects.

The output is plain text, embedded by the application with `include_str!` and handed to
`frus::licenses::add_all`. The format is length-prefixed so that a licence text can
contain anything at all:

    frus-licenses 1
    # comments, ignored
    @ <byte length of the text>
    - <package> <version>
    - <package> <version>
    .
    <exactly that many bytes of text>
"""

import argparse
import json
import os
import re
import subprocess
import sys
from collections import defaultdict

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))

# What a package's licence file is called, whatever the spelling.
LICENCE_NAMES = ("LICENSE", "LICENCE", "COPYING", "UNLICENSE", "NOTICE")
# Files that start with one of those names and are plainly not the licence.
NOT_LICENCES = (".RS", ".TOML", ".PY", ".SH", ".YML")


def cargo(args):
    """Runs cargo in the repository and hands back its stdout."""
    done = subprocess.run(
        ["cargo"] + args, cwd=ROOT, capture_output=True, text=True, encoding="utf-8"
    )
    if done.returncode != 0:
        sys.exit("cargo %s failed:\n%s" % (" ".join(args), done.stderr))
    return done.stdout


def linked_packages(package):
    """`(name, version)` of everything `package` links, on every target."""
    out = cargo(
        ["tree", "-p", package, "-e", "no-dev", "--target", "all", "--prefix", "none"]
    )
    found = set()
    for line in out.splitlines():
        line = re.sub(r" \((\*|proc-macro)\)$", "", line.strip())
        match = re.match(r"^([A-Za-z0-9_.+-]+) v([0-9][^ ]*)", line)
        if match:
            found.add((match.group(1), match.group(2)))
    return found


def licence_files(directory):
    """The licence files a package ships, in a stable order."""
    found = []
    try:
        entries = sorted(os.listdir(directory))
    except OSError:
        return found
    for name in entries:
        upper = name.upper()
        if not any(upper.startswith(n) for n in LICENCE_NAMES):
            continue
        if upper.endswith(NOT_LICENCES):
            continue
        path = os.path.join(directory, name)
        if os.path.isfile(path):
            found.append(path)
    return found


def read_text(path):
    """A licence file, with its line endings normalised and its edges trimmed.

    Nothing else is touched: the words are the package's, not ours.
    """
    with open(path, "rb") as handle:
        raw = handle.read()
    text = raw.decode("utf-8", errors="replace").replace("\r\n", "\n")
    return "\n".join(line.rstrip() for line in text.split("\n")).strip()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("-p", "--package", required=True, help="the application's crate")
    parser.add_argument("-o", "--out", required=True, help="the file to write")
    args = parser.parse_args()

    wanted = linked_packages(args.package)
    metadata = json.loads(cargo(["metadata", "--format-version", "1"]))
    packages = {(p["name"], p["version"]): p for p in metadata["packages"]}
    # A workspace crate keeps its licence at the repository root, beside the manifest
    # that declares the workspace — which is where these two live.
    workspace_files = licence_files(ROOT)

    # text -> the packages that ship exactly that text.
    notices = defaultdict(list)
    # declared expression -> the packages that ship no file at all.
    undeclared = defaultdict(list)

    for key in sorted(wanted):
        name, version = key
        package = packages.get(key)
        if package is None:
            undeclared["unknown"].append((name, version, ""))
            continue
        directory = os.path.dirname(package["manifest_path"])
        files = licence_files(directory)
        # A crate of this workspace: the licences are at the root of it.
        if not files and directory.startswith(ROOT):
            files = workspace_files
        if not files:
            spdx = package.get("license") or "not declared"
            undeclared[spdx].append((name, version, package.get("repository") or ""))
            continue
        for path in files:
            notices[read_text(path)].append((name, version))

    for spdx, listed in sorted(undeclared.items()):
        where = "\n".join(
            "  %s %s%s" % (n, v, "  %s" % r if r else "") for n, v, r in listed
        )
        notices[
            "These packages declare `%s` and ship no licence file of their own, so "
            "there is no text here to reproduce. The declaration is the package's own, "
            "from its manifest; the terms are the published text of %s.\n\n%s"
            % (spdx, spdx, where)
        ] = [(n, v) for n, v, _ in listed]

    # Sorted by the first package in each notice, so the file is stable between runs and
    # a diff shows what actually changed.
    entries = sorted(notices.items(), key=lambda kv: (sorted(kv[1])[0], kv[0][:80]))

    out = [
        "frus-licenses 1",
        "# Generated by scripts/gen_licenses.py from the dependency graph of `%s`."
        % args.package,
        "# %d packages, %d notices. Regenerate after changing a dependency; do not edit."
        % (len(wanted), len(entries)),
        "",
    ]
    for text, listed in entries:
        body = text.encode("utf-8")
        out.append("@ %d" % len(body))
        for name, version in sorted(set(listed)):
            out.append("- %s %s" % (name, version))
        out.append(".")
        out.append(text)
    blob = "\n".join(out) + "\n"

    destination = os.path.join(ROOT, args.out)
    os.makedirs(os.path.dirname(destination), exist_ok=True)
    with open(destination, "w", encoding="utf-8", newline="\n") as handle:
        handle.write(blob)

    missing = sum(len(v) for v in undeclared.values())
    print(
        "%s: %d packages, %d notices, %.1f KB"
        % (args.out, len(wanted), len(entries), len(blob.encode("utf-8")) / 1024)
    )
    if missing:
        print(
            "  %d of them ship no licence file; they are listed by declaration."
            % missing
        )


if __name__ == "__main__":
    main()
