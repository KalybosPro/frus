# Milestone 388 — A tab that can show as well as say

`TabBar` took a string per tab and drew it. A bottom-level navigation bar wants an icon
over the word, and a compact one wants the icon alone. Neither was reachable.

Three builders now: `tab`, `icon_tab`, `icon_only_tab`.

## One tall tab makes the whole row tall

The reference has two heights — 46 for an ordinary tab, 72 for one stacking an icon over a
label — and they are heights of the **row**, not of a tab.

Tabs of two heights in one bar would put their labels on two different lines, with the
indicator ruling under whichever line happened to be lower. So the question "is this bar
tall?" is asked once, of the whole row, and recorded on `TabStyle` — which the bar and
every tab already share, precisely so a measurement cannot be answered two ways.

An icon **on its own** does not raise the row. It sits in an ordinary tab, which is what
the reference does and what a compact bar wants.

## An icon-only tab still has to have a name

`icon_only_tab(icon, label, content)` takes a label it does not draw.

The reference leaves such a tab nameless and expects the caller to wrap it in something
that names it. That is a hole with a lid: it works when someone remembers. Asking for the
word at the point the tab is declared costs a parameter and cannot be forgotten, and the
semantics node carries it exactly as a text tab's does.

The test asserts both halves — nothing drawn, the name still there.

## One measurement, again

`content_width` is what a tab's content comes to: its text, its icon, or **whichever is
wider** when it has both, since the two are stacked rather than side by side.

The tab's own width comes from it, and so does the primary indicator's. The file already
carried the warning — a tab measured one way and an indicator measured another agree on
every label until they do not, and the failure is an underline creeping away from its tab
— so the new geometry went into the same function rather than beside it.

A short word under a wide icon is now a tab as wide as the icon, and the indicator matches.

## Painted as one block

The icon and the label are centred **together**, not each in its own half.

Two separately centred pieces drift apart as the text's height changes with the font, and
in a row where one tab has an icon and its neighbour does not, the two labels would no
longer sit on the same line. The block is `icon + gap + text` tall, centred once, and both
parts hang off its top.

## Left

`iconMargin` is a fixed 2 px, the reference's Material 3 figure, with no way to override
it. `Tab.height` per tab is deliberately absent — see above.
