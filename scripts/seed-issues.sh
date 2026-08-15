#!/usr/bin/env bash
#
# Creates the repository's starting issues, and the labels they use, from the
# markdown in `.github/issues/`.
#
#   ./scripts/seed-issues.sh --dry-run   # print what would be created
#   ./scripts/seed-issues.sh             # create it
#
# Needs the GitHub CLI, authenticated against this repository:
#
#   gh auth login
#
# Running it twice would create every issue twice — it deliberately does no
# deduplication, because guessing which of two similarly titled issues is "the same
# one" is a worse failure than an obvious duplicate. Check `gh issue list` first.

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
for file in .github/issues/*.md; do
    title=$(sed -n '1s/^title: //p' "$file")
    labels=$(sed -n '2s/^labels: //p' "$file")
    body=$(tail -n +4 "$file")

    if [[ -z "$title" ]]; then
        echo "  $file: no 'title:' on the first line — skipped" >&2
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
