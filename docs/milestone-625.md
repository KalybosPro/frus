# Milestone 625 — On a desktop, Select all puts the menu away

## Objective

When a field's selection bar, or a selectable text's, was used to **Select all**, frus kept the
bar open, offering what could be done with everything selected. That is what a phone does. A
desktop's context menu does something else: a choice in it is the end of it. The reference
draws the same line (`editable_text.dart:2914`): when a Select all comes from the toolbar, it
calls `hideToolbar()` on macOS, Linux and Windows, and leaves the toolbar alone on Android, iOS
and Fuchsia. Milestone 618 gave frus the desktop menu; this milestone gives it the desktop's
behaviour after a Select all.

## What was done

- In the shell's handling of the bar's buttons, a Select all on a field (or on selectable
  text, which goes the same way) now asks `select_all_hides_bar(default_target_platform())`.
  On the three desktops the bar is put away **with its handles**, as the reference's
  `hideToolbar()` takes both by default. Elsewhere nothing changes: the bar is shown again,
  where it was, with the list the new selection calls for.
- **The system decides, not the theme.** The reference reads `defaultTargetPlatform` here,
  not `Theme.of(context).platform`: this is how the system's own menus behave, not how the
  application looks. So an application themed like Android, running on Windows, closes the
  menu; one themed like Linux, running on a phone, keeps the bar.
- **A selection area's bar is unchanged.** The reference's selectable region shows its
  toolbar again after a Select all on every platform (`selectable_region.dart:1848`), and so
  does frus's.

The reference's second switch in the same place, which brings the selection's end into view
on Android, Fuchsia, Linux and Windows, is not part of this milestone.

## Tests

- `select_all_hides_the_bar_on_the_desktops`: the rule, for each of the six platforms.
- `a_desktop_s_select_all_puts_the_menu_away`: a right-click on a field opens the menu; its
  Select all closes it; a second right-click finds the text still selected (the menu offers
  Copy). The test host is always one of the three desktops. The override of the system's
  platform is process-wide, so the phone side is pinned by the unit test, not by a driver.

Mutation testing: three mutants, all killed — the rule turned to the phones, the bar left
open on a desktop, and the theme-independent system platform replaced by a fixed phone.
