# Text, colour and the theme

Every widget draws itself from the **theme**: its colours, its type, its corners. An
application chooses a theme once, and each widget can still be told otherwise. Nothing on
screen is styled only one way: what the theme says is a default, and what you write on a widget
wins.

## Text

`Text` takes its size, weight and colour from the theme's body style unless you say otherwise.

```rust
use frus::{Column, FontWeight, Text, TextAlign, Widget};

fn article() -> Box<dyn Widget> {
    Box::new(
        Column::new()
            .spacing(8.0)
            .child(Text::new("A quiet morning").size(28.0).weight(FontWeight::Bold).heading())
            .child(Text::new("Published at 06:30").size(13.0).italic())
            .child(
                Text::new("The first paragraph, long enough to wrap onto several lines.")
                    .wrap()
                    .align(TextAlign::Justify),
            )
            .child(Text::new("Read more").underline()),
    )
}
```

`.heading()` also tells screen readers that the line is a heading. `.max_lines(2).ellipsis()`
cuts long text with "…"; `Text::new(..).selectable()` lets the reader select and copy it.

The theme's **type roles** — `display_large` down to `label_small` — are what a screen should
use for its titles and body text, so that every screen of the application agrees:

```rust
use frus::{component, Column, Text, Widget};

fn titled() -> Box<dyn Widget> {
    Box::new(component(|cx| {
        let text = &cx.theme().text;
        Box::new(
            Column::new()
                .child(Text::styled("Settings", text.headline_small))
                .child(Text::styled("Choose how the application looks.", text.body_medium)),
        )
    }))
}
```

## Colours from the theme

`cx.theme().scheme` holds the colour **roles**: `primary` and `on_primary` (what is drawn on
top of it), `surface`, `error`, `outline_variant`, and the rest. A widget built from roles
looks right in the light theme, the dark theme and any theme the application makes:

```rust
use frus::{component, Container, Text, Widget};

fn notice() -> Box<dyn Widget> {
    Box::new(component(|cx| {
        let scheme = &cx.theme().scheme;
        Box::new(
            Container::new()
                .padding(12.0)
                .radius(8.0)
                .color(scheme.primary_container)
                .child(Text::new("Saved").color(scheme.on_primary_container)),
        )
    }))
}
```

Write a colour by hand — `Color::hex("#6366F1")`, `Color::rgb8(99, 102, 241)`,
`Colors::WHITE` — only where it is meant to stay the same whatever the theme.

## The application's theme

`Theme::from_seed(colour, dark)` makes a whole scheme from one colour. Give the application a
light theme, a dark one, and let the system choose:

```rust
use frus::{text, Color, FrusApp, Theme, ThemeMode};

fn app() -> FrusApp {
    let brand = Color::hex("#0F766E");
    FrusApp::from_fn(|_cx| Box::new(text("Hello")))
        .title("Tides")
        .theme(Theme::from_seed(brand, false))
        .dark_theme(Theme::from_seed(brand, true))
        .theme_mode(ThemeMode::System)
}
```

The application hands the result to `frus::main!(app())`. While it runs,
`host::app().set_theme_mode(ThemeMode::Dark)` switches it — from a settings switch, say.

## Changing how one kind of widget looks, everywhere

`theme.widgets` holds the defaults of each kind of widget: buttons, fields, cards, app bars,
dialogs, and the rest. Set a field and every widget of that kind follows, unless the widget
itself says otherwise:

```rust
use frus::{BorderRadius, Color, Theme};

fn square_buttons() -> Theme {
    let mut theme = Theme::from_seed(Color::hex("#0F766E"), false);
    theme.widgets.button.radius = Some(BorderRadius::uniform(6.0));
    theme.widgets.button.height = Some(44.0);
    theme
}
```

## Changing one widget

What you write on a widget wins over the theme:

```rust
use frus::{Button, Color, Colors, Icon, Icons, Variant, Widget};

fn buttons() -> Vec<Box<dyn Widget>> {
    vec![
        Box::new(Button::new("Save").on_press(|| {})),
        Box::new(Button::new("Cancel").variant(Variant::Outlined).on_press(|| {})),
        Box::new(Button::new("Delete").variant(Variant::Danger).on_press(|| {})),
        Box::new(Button::new("Add").icon(Icon::new(Icons::ADD)).variant(Variant::Tonal).on_press(|| {})),
        Box::new(
            Button::new("Custom")
                .color(Color::hex("#EA580C"))
                .label_color(Colors::WHITE)
                .radius(4.0)
                .on_press(|| {}),
        ),
    ]
}
```

A button with no `on_press` is disabled, and looks it.

When a look depends on the state — pressed, hovered, focused, disabled — a `ButtonStyle` says
it state by state, and `WidgetStateProperty::all` is the same value in every state:

```rust
use frus::{Button, ButtonStyle, Color, Widget, WidgetStateProperty};

fn styled() -> Box<dyn Widget> {
    Box::new(
        Button::new("Styled")
            .style(
                ButtonStyle::new()
                    .background_color(WidgetStateProperty::all(Color::hex("#1E293B")))
                    .foreground_color(WidgetStateProperty::all(Color::hex("#F8FAFC"))),
            )
            .on_press(|| {}),
    )
}
```

## A different theme for one part of the screen

`Themed` gives a subtree a theme of its own — a dark panel in a light page, a branded card:

```rust
use frus::{Button, Color, Themed, Theme, Widget};

fn dark_panel() -> Box<dyn Widget> {
    Box::new(Themed::new(
        Theme::from_seed(Color::hex("#7C3AED"), true),
        Button::new("Inside a dark theme").on_press(|| {}),
    ))
}
```

## Icons and pictures

`Icon::new(Icons::NAME)` draws one of the bundled Material symbols; `.size()` and `.color()`
change it. `Image::network(url)` shows a picture from the network,
`Image::memory(bytes)` one already in memory, and `asset!("photo.png")` one embedded in the
program from a file beside the source. `.fit(BoxFit::Cover)` fills its box.

```rust
use frus::{Colors, Icon, Icons, Image, Row, Widget};

fn icons_and_a_picture() -> Box<dyn Widget> {
    Box::new(
        Row::new()
            .spacing(8.0)
            .child(Icon::new(Icons::FAVORITE).color(Colors::RED).size(32.0))
            .child(Image::network("https://example.com/photo.png").size(96.0, 96.0)),
    )
}
```
