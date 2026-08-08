# Jalon 61 — Fully customisable AppBar/NavBar

A standing directive from the user: *frus's AppBar is a different design — but
**everything about it must be customisable by the application***. Milestone 60
had just **hard-coded** the medium weight of AppBar/NavBar titles: precisely the
counter-example. This milestone fixes that and establishes the rule: **private
constants are defaults only, never the only option**.

## `AppBar`: every decision has an override

frus's *adaptive* design (automatic folding into overflow according to width)
stays — what changes is the **degree of customisation**:

- **`title_style(TextStyle)`** — the title's size/weight/italic/colour (default:
  20 px medium, the theme's colour). The budget measurement follows the style.
- **`title_widget(impl Widget)`** — the title becomes an **arbitrary widget**
  (a logo, a composed row…). Its declared width feeds the folding budget.
- **`action_widget(impl Widget)`** — a **free widget** action (badge, avatar,
  field…), inserted in order. Always **inline**: an arbitrary widget cannot fold
  into a text menu row — labelled actions remain the only foldable ones (that is
  the adaptive design's contract).
- **`action_size(f32)`**, **`gap(f32)`** — sizes and spacing (defaults 16/8).
- **`background(Color)`**, **`height(f32)`** — optional chrome (default: a bare
  row, the parent decides).
- `leading` was already a free widget slot.

The folding algorithm is generalised: the widths of free widgets are always
counted (they are always inline), and labelled actions fold from the front, in
order. **The default behaviour is unchanged** — the two existing folding tests
pass as they are.

## `NavBar`: likewise

- **`title_style(TextStyle)`** (default: 20 px medium; the style's colour wins
  over the theme's when specified).
- **`height(f32)`** (default: 56 px).

## The rule this establishes (project memory)

Every visual or structural decision a widget makes must have an override path:
builders with themed defaults, and `impl Widget` slots wherever a slot makes
sense. To be applied to the existing widgets as we go, and to every new widget.

## Validation

- `frus-widgets`: **136 tests** (+4: overridden title style, a title widget
  replacing the text, an action widget never folded even when cramped, NavBar
  style and height overridden).
- Default behaviour unchanged: the existing folding tests green, demo 15, all 15
  suites green. A warning-free build.

## What's next

- Propagate the same customisation audit to the other composed widgets
  (Scaffold, NavRail/BottomBar, Drawer, BottomSheet…), as we go.
- Pick the typography thread back up: `TextSpan` + `TextLayout` (cosmic-text).
