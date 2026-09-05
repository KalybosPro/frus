# Milestone 466 — Four buttons whose meaning is fixed and whose name is not

`BackButton`, `CloseButton`, `DrawerButton`, `EndDrawerButton` — the reference calls them
action buttons (`action_buttons.dart`), and they are all one idea: an `IconButton` that
already knows its glyph and takes its name from the framework's own words.

None of them existed here. What did exist was this, at its single call site:

```rust
crate::IconButton::glyph("←")
    .label(crate::localizations::of().back_button_label())
```

## Three things wrong with a character

**It is a codepoint.** U+2190 renders in whatever the loaded font has for it, or in
nothing. Its weight is the font's, its optical size is the font's, and its baseline is a
text baseline — beside a drawn 24-grid glyph it will be the wrong weight and very
slightly the wrong height, and no amount of `icon_size` fixes either.

**It is the same arrow everywhere.** The reference picks by platform: a chevron where the
platform's own back control is a chevron, an arrow everywhere else
(`action_buttons.dart:132`). One character cannot be two shapes.

**It was named by hand at the call site.** Which worked, once, in the one place somebody
remembered. `Scaffold` has a drawer and no button that opens it by name; a dialog has a
cross and nothing that says "Close" to a reader.

`Icons::ArrowLeft` is new — a triangular head and a bar as two overlapping filled
subpaths, the way `cross_x` is two diagonals. Two shapes rather than one traced outline
because the union is what is wanted and the non-zero rule gives it for free, where one
outline needs eight points placed by hand and is wrong the first time somebody changes
the bar's thickness.

## The words, twice

Each button puts its label in **both** places: the semantics, where a screen reader finds
it, and a `Tooltip`, where a pointer finds it. That is what the reference does
(`action_buttons.dart:54`), and the alternative is a control named for one of its two
audiences.

This is the first thing in the crate to use milestone 462's `Tooltip` and milestone 449's
`Localizations` together, which is what they were both for.

`open_drawer_label` is new on the table. **One word for both edges**, as the reference
has it (`action_buttons.dart:331` against `:362`): a reader told which edge a panel comes
in from is being told about the layout rather than about the action. There is a test whose
only job is to hold that, because it looks like an oversight.

## `ActionIconTheme`

Four `Option<Icons>`, the reference's `action_icons_theme.dart`. An application with its
own icon set should not have to leave four of the framework's showing through.

## The macro, again

The four faces differ only in which `Kind` they carry, so they wanted a `macro_rules!` —
and milestone 465, one milestone ago, had just learned why they cannot have one:
`every_control_with_an_enabled_flag_honours_all_four` reads the **source** of every module
with an `enabled` flag and cannot parse a hook inside a macro body.

So they are typed out, with the reason above them. Twice in two milestones is a pattern
rather than an accident, and it is now written where the next person will read it before
folding them back up.

## The tests

Five, four of which fail when the milestone is undone — checked by taking the reader's
language away, giving the end drawer its own word, ignoring the theme's glyph, and putting
the character back.

`the_four_are_named_in_the_reader_s_language` installs a French table and asserts the
button says *Retour*. It is the test the character version would have passed, since the
label was already localized there — what it would not have passed is the tooltip half,
which is why the assertion reads the semantics of the **whole built frame** rather than
the button's own.

## The pictures

**Two moved.** `icon_set` gained the arrow — fourteen icons now, and thirty more pixels of
frame to hold it. `navigation_chrome` is the back button itself, and it took two goes: the
first arrow had a head nine wide and nineteen tall on a twenty-four grid, which reads as a
triangle with a stub attached rather than as an arrow. Eight by sixteen, with the bar
starting inside the head and stopping short of the right edge, is what the picture said it
wanted. **That is the whole argument for reading the goldens rather than blessing them**:
the geometry compiled, the tests passed, and it was wrong.
