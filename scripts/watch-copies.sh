#!/usr/bin/env bash
# watch-copies.sh — look for unlicensed copies of frus elsewhere on the internet.
#
# You cannot see who *clones* a public repo (GitHub keeps that anonymous), but a
# clone only matters once someone *republishes* the code. This script hunts for
# that: it searches public GitHub code for identifiers that are unique to frus,
# and crates.io for name collisions. Any hit outside your own repo is worth a
# look — it may be a fork that stripped your LICENSE/NOTICE (an Apache-2.0
# violation), or a rename like the "frust" project.
#
# Requires: gh (authenticated), curl. Run: bash scripts/watch-copies.sh
set -u

OWN_REPO="frus-rs/frus"

# Identifiers distinctive enough that a hit almost certainly came from frus.
# VERIFIED to return zero external hits on 2026-11-02 — a new hit is a signal.
# Do NOT add common names here: `paint_elevation` and `takes_typing`, for
# example, occur in hundreds of unrelated projects and only create noise.
# To add a token, first run:  gh api -X GET search/code -f q='"TOKEN" -repo:frus-rs/frus' --jq .total_count
# and only keep it if that prints 0.
TOKENS=(
  "QUICK_DECODE_PIXELS"
  "frus_core"
  "frus::authoring"
)

# Your handle/email. Hits are usually your OWN other repos, not theft —
# listed separately so you can tell them apart at a glance.
IDENTITY=(
  "kalyboskokou1"
)

# Verbatim prose unique to your docs/README. Text is copied more often than
# code, but only add lines that are genuinely yours and genuinely distinctive
# (check with the gh command above first). Empty by default.
PHRASES=()

# crate names to watch for squatting / confusing forks on crates.io
CRATE_NAMES=( "frus" "frust" "frus-core" "frus-widgets" )

alerts=0

say() { printf '%s\n' "$*"; }
hr()  { printf -- '----------------------------------------------------------\n'; }

search_github() {
  local term="$1"
  # URL-encode via gh's -f (it handles it); exclude our own repo.
  local json count
  # gh prints an API error on stdout, so its exit status is what tells a failure from no hits.
  if ! json=$(gh api -X GET search/code \
           -f q="\"$term\" -repo:$OWN_REPO" \
           --jq '{total: .total_count, hits: [.items[]? | .repository.full_name] | unique}' \
           2>/dev/null); then
    say "  ! search failed or rate-limited for: $term"
    alerts=$((alerts + 1))
    return
  fi
  count=$(printf '%s' "$json" | grep -o '"total":[0-9]*' | grep -o '[0-9]*')
  if [ "${count:-0}" -gt 0 ]; then
    say "  ⚠  $count external hit(s) for «$term»:"
    alerts=$((alerts + 1))
    printf '%s' "$json" | grep -o '"[^"]*/[^"]*"' | sort -u | sed 's/^/       /'
  else
    say "  ok  no external code hits for «$term»"
  fi
  sleep 7   # GitHub search: 30 requests/min
}

check_crate() {
  local name="$1" body created repo
  body=$(curl -s --max-time 20 -H 'User-Agent: frus-watch' \
         "https://crates.io/api/v1/crates/$name")
  if printf '%s' "$body" | grep -q '"errors"'; then
    say "  ok  no crate named «$name»"
    return
  fi
  created=$(printf '%s' "$body" | grep -o '"created_at":"[^"]*"' | head -1 | cut -d'"' -f4)
  repo=$(printf '%s' "$body" | grep -o '"repository":"[^"]*"' | head -1 | cut -d'"' -f4)
  say "  ⚠  crate «$name» exists  (created $created)  repo: ${repo:-?}"
}

hr; say "GitHub code search (identifiers unique to frus)"; hr
for t in "${TOKENS[@]}"; do search_github "$t"; done
# Only the identifiers decide the exit status: the other sections always have known hits.
token_alerts=$alerts

hr; say "GitHub code search (your handle — hits are usually your own repos)"; hr
for i in "${IDENTITY[@]}"; do search_github "$i"; done

if [ "${#PHRASES[@]}" -gt 0 ]; then
  hr; say "GitHub code search (verbatim doc prose)"; hr
  for p in "${PHRASES[@]}"; do search_github "$p"; done
fi

hr; say "crates.io name watch"; hr
for c in "${CRATE_NAMES[@]}"; do check_crate "$c"; done

hr
say "Done. A hit is not proof of theft — open the repo and check whether it"
say "kept your LICENSE and NOTICE. If a redistribution stripped them, that is"
say "an Apache-2.0 / MIT violation you can act on (GitHub DMCA, or a takedown)."
[ "$token_alerts" -eq 0 ]
