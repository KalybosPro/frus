# The repository's starting issues

One file per issue. `../../scripts/seed-issues.sh` reads them, creates the labels they
use, and opens them with the GitHub CLI:

```sh
gh auth login                        # once
./scripts/seed-issues.sh --dry-run   # see what it would do
./scripts/seed-issues.sh             # do it
```

On **Windows**, from PowerShell, use the wrapper — PowerShell will not run a `.sh`,
and it does not say so: `./scripts/seed-issues.sh` there returns silently having done
nothing.

```powershell
.\scripts\seed-issues.ps1 --dry-run
.\scripts\seed-issues.ps1
```

Re-running is safe: an issue whose exact title is already open is skipped.

They live in the repository rather than only on GitHub for two reasons: they can be
reviewed in a pull request like anything else, and a fresh fork or a mirror starts with
its work already described instead of an empty issue tracker.

## The format

```
title: One line, in the imperative
labels: good first issue, documentation
<blank line>
The body, in GitHub-flavoured markdown.
```

The script takes line 1 as the title, line 2 as a comma-separated label list, and
everything from line 4 as the body. `seed-issues.sh` skips any file without a `title:`,
which is what keeps this README out of your issue tracker.

## Writing one

The point of an issue here is to let somebody who has never read this codebase finish
the work without asking three questions first. So each one says:

- **why it matters** — what is worse today because it is not done;
- **where to look** — the crate, the file, the function;
- **how to know it is finished** — a test that passes, a number that drops, a command
  that runs;
- **what not to do**, where there is an obvious wrong turn.

Issues that only say what is missing are cheap to write and no help to anybody.
