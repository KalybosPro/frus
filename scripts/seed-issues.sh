#!/usr/bin/env bash
#
# Creates the repository's starting issues, and the labels they use, from the
# markdown in `.github/issues/`.
#
#   ./scripts/seed-issues.sh --dry-run   # print what would be created
#   ./scripts/seed-issues.sh             # create it
#
# This is a **bash** script. On Windows run it from Git Bash, or use the PowerShell
# wrapper beside it: PowerShell does not execute a `.sh`, and it does not say so —
# `./scripts/seed-issues.sh` there returns silently, having done nothing.
#
#   bash ./scripts/seed-issues.sh        # from PowerShell or cmd
#   ./scripts/seed-issues.ps1            # the wrapper, same arguments
#
# Needs the GitHub CLI, authenticated against this repository:
#
#   gh auth login
#
# Safe to re-run: an issue whose **exact title** is already open is skipped. That is a
# title match and not a judgement about what two issues have in common — a near
# duplicate is still yours to spot.

set -euo pipefail

cd "$(dirname "$0")/.."

DRY_RUN=0
[[ "${1:-}" == "--dry-run" ]] && DRY_RUN=1

# A dry run deliberately needs neither the CLI nor an account: reading back what is
# about to be posted to a public tracker should not itself require credentials.
if (( ! DRY_RUN )); then
    if ! command -v gh >/dev/null 2>&1; then
        echo "The GitHub CLI (gh) is not installed: https://cli.github.com" >&2
        echo "Run with --dry-run to see what this would create." >&2
        exit 1
    fi
    if ! gh auth status >/dev/null 2>&1; then
        echo "gh is not authenticated. Run: gh auth login" >&2
        exit 1
    fi
fi

# name|colour|description. The first two mirror the roadmap's own tags, so that a
# label and a roadmap line mean the same thing.
LABELS=(
    "good first issue|7057ff|Self-contained, clear success criterion, no deep context needed"
    "help wanted|008672|Meaty, well-scoped, needs some familiarity with a subsystem"
    "design first|d93f0b|Open a discussion before writing code — the shape is not settled"
    "bug|d73a4a|Something behaves differently from how it is documented"
    "documentation|0075ca|Prose, examples, rustdoc"
    "testing|fbca04|Tests and the golden-image suite"
    "ci|fef2c0|Continuous integration and the checks it runs"
    "build|c5def5|Packaging, dependencies, size, toolchain"
    "performance|a2eeef|Measured, with a benchmark to show for it"
    "rendering|5319e7|The scene, the GPU pipeline, text"
    "widgets|1d76db|The widget library and the interaction model"
    "web|bfd4f2|The wasm target"
    "android|3d8f3d|The Android target"
    "platform|006b75|A new platform, or the shell layer"
    "accessibility|0e8a16|Semantics, screen readers"
)

echo "== Labels"
for entry in "${LABELS[@]}"; do
    IFS='|' read -r name colour description <<<"$entry"
    if (( DRY_RUN )); then
        echo "  would ensure: $name"
    else
        # `--force` updates the colour and description of a label that already
        # exists rather than failing, which is what makes this safe to re-run.
        gh label create "$name" --color "$colour" --description "$description" --force
    fi
done

echo
echo "== Issues"

# The titles already open, so that re-running this adds what is new instead of a
# second copy of everything. One call, not one per issue.
existing=""
if command -v gh >/dev/null 2>&1 && gh auth status >/dev/null 2>&1; then
    existing=$(gh issue list --state open --limit 200 --json title --jq '.[].title' 2>/dev/null || true)
fi

for file in .github/issues/*.md; do
    title=$(sed -n '1s/^title: //p' "$file")
    labels=$(sed -n '2s/^labels: //p' "$file")
    body=$(tail -n +4 "$file")

    if [[ -z "$title" ]]; then
        echo "  $file: no 'title:' on the first line — skipped" >&2
        continue
    fi

    # `grep -Fx`: fixed strings, whole line. A title full of backticks and
    # parentheses is not a regular expression.
    if [[ -n "$existing" ]] && printf '%s\n' "$existing" | grep -qFx "$title"; then
        echo "  already open: $title"
        continue
    fi

    label_args=()
    IFS=',' read -ra parts <<<"$labels"
    for part in "${parts[@]}"; do
        # Trim the spaces around a comma-separated list.
        label_args+=(--label "$(echo "$part" | sed 's/^ *//; s/ *$//')")
    done

    if (( DRY_RUN )); then
        echo "  would create: $title  [${labels}]"
    else
        gh issue create --title "$title" --body "$body" "${label_args[@]}"
    fi
done

echo
echo "Done. Newcomers land here:"
echo "  https://github.com/KalybosPro/frus/labels/good%20first%20issue"
