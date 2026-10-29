# Milestone 712 — The web checks check what frus does today

## Objective

Milestone 711 left a failure behind: the selection bar's browser check
(`scripts/web-selection-bar-check.py`, milestone 568) failed two lines — after **Select all**
was activated through the accessibility tree, the bar did not come back offering Cut and Copy —
and failed them the same way on master. It was written down as an older break in frus. This
milestone found where the break was, and ran every web check until each one passed again, over
and over.

## The selection bar: the check was out of date, not frus

Milestone 625 gave the desktops their own behaviour after a Select all: a desktop's context
menu closes once a choice is made in it, as the reference's does (`hideToolbar()` on macOS,
Linux and Windows), and a phone's bar stays, offering what can be done with everything
selected. On the web the platform is the one the browser runs on (milestone 614), and the check
runs in a browser on Windows — a desktop. frus closed the menu, as it should; the check, written
before that rule, still expected the phone's bar.

The check now runs its steps twice in the same browser:

- **on the desktop it runs on**: Select all puts the menu away; the context-menu key opens it
  again, on everything still selected — Cut, Copy and Paste, and no Select all;
- **told it is a phone** (the user agent and `navigator.platform` of an Android phone, set
  before the page loads, which is what `TargetPlatform::from_browser` reads): the bar stays
  open, now offering Cut, Copy and Paste.

The phone's run passing is also what shows the override took: the desktop's run closes the bar.

## The address check had been failing since the demo changed

`scripts/web-address-check.py` (milestone 561) failed on master, every time: its last part
clicked at fixed coordinates — the add field, a task's avatar, the task screen's Back button —
taken from the demo as it was drawn in September. The demo has changed since, the clicks landed
elsewhere, and no task screen was ever opened. Those steps now find the widgets by name in the
accessibility tree (*New task*, the avatar's initials *BM*, *Back*) and press them as a screen
reader would, the way the other web checks already did.

Fixed, it still failed about one run in two: no page found, or a back and forward whose screen
did not change. The check opened a new tab for each of its three parts and never closed the
previous ones, and a headless page beside another is the one put in the background, where it
draws nothing. Each part now opens the only tab.

## Every web check in the foreground

Milestone 711 found that headless Edge sometimes puts the page in the background — hidden, its
frames stopped — so a key the shell handles shows nothing, and gave its own check the flags and
the focus emulation that prevent it. The shared helper (`web-accessibility-check.py`) now holds
them: `FOREGROUND`, the flags every check starts its browser with, and `open_at`, which turns
focus emulation on. The five checks that use the helper take both; the address check, which
has its own, takes the same.

The text-editing check read the accessibility tree once, 0.6 seconds after the Enter that adds
a task, and failed once in a dozen runs with the field already empty — the new row had not
reached the tree yet. It now waits for the row, up to three seconds.

## Checked

All six web checks — the accessibility bridge, the caret keys, the browser shortcuts, text
editing, the selection bar on both platforms, the addresses — against the demo built from
master, in headless Edge:

| Check | Runs | Passed |
|---|---|---|
| `web-accessibility-check.py` | 7 | 7 |
| `web-caret-keys-check.py` | 7 | 7 |
| `web-keys-check.py` | 7 | 7 |
| `web-text-editing-check.py` | 20 | 8 in 8 with the wait (11 in 12 before it) |
| `web-selection-bar-check.py` | 8 | 8 |
| `web-address-check.py` | 14 since each part has the only tab | 13 — the fourteenth lost its DevTools connection mid-run, a failure of the driver, not a line |

## Not changed

frus itself: no Rust code changes in this milestone. The behaviour the selection bar's check
now expects is milestone 625's, and its unit and driver tests already pinned it.
