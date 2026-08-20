# Milestone 367 — The names the reference uses

Twenty widgets were the reference's widget under a different name. `TextInput` for a text
field, `List` for a list view, `Toast` for a snack bar, `NavBar`, `NavRail`, `Spinner`,
`Collapsible`, `Popover`. Every one of them the same thing, spelled our way.

## Why a name is not a detail here

The whole proposition of this framework is that someone who knows the reference already
knows it. A different name breaks that at the first line of code somebody writes: they type
what they know, it does not exist, and they have to go and look — for a widget that was
there all along under a synonym. The cost is paid by every reader, forever, and it buys
nothing.

It also breaks search. Somebody looking for how to limit a text field's length searches for
`TextField`, and a framework that spells it `TextInput` is not in the results.

The project's own standing rule already covered this: *if we have done something that does
not follow the reference, we correct it against the reference*. A public name is the most
visible thing there is to get wrong.

## The map

| Ours | The reference's |
|---|---|
| `TextInput`, `TextInputStyle`, `TextInputVariant` | `TextField`, `TextFieldStyle`, `TextFieldVariant` |
| `List` | `ListView` |
| `Grid` | `GridView` |
| `Scroll` | `SingleChildScrollView` |
| `Collapsible` | `ExpansionTile` |
| `Alert` | `AlertDialog` |
| `Toast`, `ToastKind`, `ToastPosition`, `ToastHost` | `SnackBar`, `SnackBarKind`, `SnackBarPosition`, `ScaffoldMessenger` |
| `Spinner` | `CircularProgressIndicator` |
| `ProgressBar` | `LinearProgressIndicator` |
| `SegmentedControl` | `SegmentedButton` |
| `NavBar` | `NavigationBar` |
| `NavRail` | `NavigationRail` |
| `Tabs`, `TabsVariant`, `TabsTheme` | `TabBar`, `TabBarVariant`, `TabBarTheme` |
| `Avatar` | `CircleAvatar` |
| `Dropdown` | `DropdownButton` |
| `Menu` | `PopupMenuButton` |
| `Popover` | `MenuAnchor` |
| `Refresh` | `RefreshIndicator` |
| `Barrier` | `ModalBarrier` |
| `Portal` | `OverlayPortal` |
| `Carousel` | `CarouselView` |

The theme field `widgets.text_input` becomes `widgets.text_field` and `widgets.tabs`
becomes `widgets.tab_bar`, so the theme reads the same way as the widget it configures.

Twelve widgets keep their names because the reference has nothing to match them to:
`Kanban`, `Timeline`, `Steps`, `Breadcrumb`, `Pagination`, `Rating`, `Kbd`, `Skeleton`,
`ColorPicker`, `Tree`, `TwoPane`, `NavScaffold`.

## Four things that had to be excluded by hand

A blind rename is wrong in four places, and each of them is a class rather than a one-off:

**`IconName::Menu`** is an icon called "menu" — three lines and a hamburger. Renaming it
`IconName::PopupMenuButton` would have been renaming a picture after a widget it has
nothing to do with.

**Labels in quotes** — `.label("Menu")`, a demo heading reading `"Grid"` — are words on a
screen, not names in an API. The rename regexes refuse a match sitting between quotes.

**`Role::TextInput` and `Role::ProgressBar`** are accessibility roles, and the vocabulary
there belongs to the platform's screen reader rather than to this framework. They stay.

**`Drag::Scroll`** in the shell is an internal state — *a scroll drag is in progress* —
and has no more to do with the widget than the word "scroll" does.

The one genuine collision was `Tabs`: the module already had a private `TabBar`, the strip
of tabs inside the composite. The composite takes the public name, since `TabBar` is what
somebody types, and the strip is now `TabStrip`. Ours carries its panel where the reference
splits `TabBar` from `TabBarView`; that difference is documented where it is made.

## Not renamed

The module files. `textinput.rs` still holds `TextField`, `scroll.rs` still holds
`SingleChildScrollView`. Modules here are private and named for the concept, not the type,
and moving twenty files would have churned every `crate::module::` doc link for no reader's
benefit.
