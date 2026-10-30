# Milestone 725 — The menu bar beside a cutout

## Objective

The first use of a simulated device found two things wrong (milestone 722, an iPhone 15 on its
side). The demo's *File View Go* menu bar showed on a phone, and it started under the screen's
left corner, where the cutout is. A real iPhone on its side would show both.

## What was done

- **`WindowMenuBar` keeps clear of the side intrusions.** Off the title bar's line (milestone
  642), the bar already started below the top intrusion, a tablet's status bar. Now its menus
  also start clear of the left one and end clear of the right one: a phone on its side has its
  cutout and its rounded corners there. The bar's colour still goes under them, as an app bar's
  does, and the page below is still told about them. On the title bar's line nothing changes:
  the system has the window's edges there.
- **The demo shows its menu bar on a wide desktop window only.** It chose by width alone
  (`SizeClass::Expanded`, 840 and up), so a phone on its side (852) or an iPad got a desktop
  menu bar. It now also asks for a desktop platform (`default_target_platform().is_desktop()`),
  which a simulated device, or the `o` key of `frus run`, changes. A phone keeps its header's
  "⋯" menu, as at any narrow width.

## Tests

- `titlebar::beside_a_cutout_the_menus_keep_clear_of_it`: insets of 59 on the left and 40 on the
  right; *File* starts at 59 or after; the bar's surface spans the whole width; the page is still
  told 59 and 40. With the old padding back, it fails.
- The demo's own menu bar tests run on a desktop platform, and still pass.
