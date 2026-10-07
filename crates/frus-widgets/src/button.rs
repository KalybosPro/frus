//! [`Button`]: the reference's buttons — elevated, filled, filled tonal, outlined and text
//! (`material/button_style_button.dart`), in one widget with a variant.
//!
//! They share everything but their colours and their elevation: any widget as their
//! content, or a label, with an icon beside it if asked; a minimum size of 64 × 40 moved by
//! the theme's visual density; padding that shrinks as the reader's text grows; a 48 px
//! touch target around a smaller button; a stadium; and a style resolved **per state** —
//! the caller's [`ButtonStyle`], then the theme's for that kind of button, then the
//! reference's defaults (milestone 632).
//!
//! ```ignore
//! Button::new("Save").on_press(Msg::Save)                              // filled
//! Button::new("Cancel").variant(Variant::Text).on_press(Msg::Cancel)  // text
//! Button::with_child(row![...]).variant(Variant::Outlined)             // any content
//! Button::new("Send").icon(Icon::new(Icons::SEND)).on_press(Msg::Send) // icon and label
//! ```

use frus_core::{
    Alignment, BorderRadius, BorderSide, Color, Insets, Rect, Scene, ShapeBorder, Size, TextStyle,
};
use frus_layout::{Align, Dimension, FlexDirection, Justify, Style};

use crate::interaction::Status;
use crate::theme::{TapTarget, Theme};
use crate::widget::Widget;
use crate::widgetstate::{WidgetState, WidgetStateProperty, WidgetStates};

/// A button's minimum height at the standard density (`elevated_button.dart:593`).
pub const BUTTON_HEIGHT: f32 = 40.0;
/// The narrowest a button gets at the standard density, however short its label.
pub const BUTTON_MIN_WIDTH: f32 = 64.0;
/// The room either side of the label (`elevated_button.dart:458`).
pub const BUTTON_PADDING: f32 = 24.0;
/// The room either side of a **text** button's label (`text_button.dart:442`).
pub const BUTTON_TEXT_PADDING: f32 = 12.0;
/// How far an elevated button sits off the surface at rest.
pub const BUTTON_ELEVATION: f32 = 1.0;
/// An outlined button's outline.
pub const BUTTON_BORDER_WIDTH: f32 = 1.0;
/// An icon's side inside a button (`elevated_button.dart:599`).
pub const BUTTON_ICON_SIZE: f32 = 18.0;

/// How much of a screen's attention a button is asking for: which of the reference's
/// buttons it is.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Default)]
pub enum Variant {
    /// The accent, filled — the reference's `FilledButton`: the one action a screen is
    /// about.
    #[default]
    Filled,
    /// A tonal fill — `FilledButton.tonal`: beside a filled button, not competing with it.
    Tonal,
    /// A raised surface with a shadow — `ElevatedButton`, for a button over busy content.
    Elevated,
    /// An outline and no fill — `OutlinedButton`.
    Outlined,
    /// A label alone — `TextButton`.
    Text,
    /// Filled in the **error** role — a destructive action.
    ///
    /// Not one of the reference's five: there, a destructive button is a filled button
    /// given the error colours by hand. It is here because saying *this action destroys
    /// something* is worth a name.
    Danger,
}

/// **Which side of the label an icon goes** (`button_style_button.dart:55`).
#[derive(Copy, Clone, Debug, PartialEq, Eq, Default)]
pub enum IconAlignment {
    /// Before the label: left in a left-to-right layout.
    #[default]
    Start,
    /// After it.
    End,
}

/// **How a button looks in each state** — the reference's `ButtonStyle`
/// (`material/button_style.dart`).
///
/// Every field is optional: what is unset falls through to the theme's style for that
/// kind of button ([`ButtonTheme`](crate::ButtonTheme)), then to the reference's defaults.
/// A [`WidgetStateProperty`] that has no answer for a state falls through the same way,
/// so a style that only says what a button looks like while enabled keeps the default
/// disabled look.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ButtonStyle {
    /// The label's type; its colour is the foreground's.
    pub text_style: Option<WidgetStateProperty<TextStyle>>,
    /// The surface.
    pub background_color: Option<WidgetStateProperty<Color>>,
    /// The label's colour, and the icon's unless `icon_color` says otherwise.
    pub foreground_color: Option<WidgetStateProperty<Color>>,
    /// The highlight under a pointer or the keyboard's focus, and the ripple of a press.
    pub overlay_color: Option<WidgetStateProperty<Color>>,
    /// The shadow's colour.
    pub shadow_color: Option<WidgetStateProperty<Color>>,
    /// How far it sits off the surface.
    pub elevation: Option<WidgetStateProperty<f32>>,
    /// The room between the edge and the content.
    pub padding: Option<WidgetStateProperty<Insets>>,
    /// The smallest it is, before the visual density.
    pub minimum_size: Option<WidgetStateProperty<Size>>,
    /// The size it is, within the minimum and maximum. An infinite side is left free.
    pub fixed_size: Option<WidgetStateProperty<Size>>,
    /// The largest it is.
    pub maximum_size: Option<WidgetStateProperty<Size>>,
    /// The icon's colour.
    pub icon_color: Option<WidgetStateProperty<Color>>,
    /// The icon's side.
    pub icon_size: Option<WidgetStateProperty<f32>>,
    /// The outline.
    pub side: Option<WidgetStateProperty<BorderSide>>,
    /// The shape.
    pub shape: Option<WidgetStateProperty<ShapeBorder>>,
    /// How compact it is. Unset, the theme's.
    pub visual_density: Option<crate::VisualDensity>,
    /// How much room it reserves for a finger. Unset, the theme's.
    pub tap_target: Option<TapTarget>,
    /// Where the content sits inside it.
    pub alignment: Option<Alignment>,
    /// Which side of the label an icon goes.
    pub icon_alignment: Option<IconAlignment>,
}

impl ButtonStyle {
    /// A style that says nothing.
    pub fn new() -> Self {
        Self::default()
    }

    /// The same, its surface `property`.
    #[must_use]
    pub fn background_color(mut self, property: WidgetStateProperty<Color>) -> Self {
        self.background_color = Some(property);
        self
    }

    /// The same, its label `property`.
    #[must_use]
    pub fn foreground_color(mut self, property: WidgetStateProperty<Color>) -> Self {
        self.foreground_color = Some(property);
        self
    }

    /// The same, its highlight and ripple `property`.
    #[must_use]
    pub fn overlay_color(mut self, property: WidgetStateProperty<Color>) -> Self {
        self.overlay_color = Some(property);
        self
    }

    /// The same, its elevation `property`.
    #[must_use]
    pub fn elevation(mut self, property: WidgetStateProperty<f32>) -> Self {
        self.elevation = Some(property);
        self
    }

    /// The same, its padding `property`.
    #[must_use]
    pub fn padding(mut self, property: WidgetStateProperty<Insets>) -> Self {
        self.padding = Some(property);
        self
    }

    /// The same, at least `size`.
    #[must_use]
    pub fn minimum_size(mut self, size: Size) -> Self {
        self.minimum_size = Some(WidgetStateProperty::all(size));
        self
    }

    /// The same, `size` exactly where it is finite.
    #[must_use]
    pub fn fixed_size(mut self, size: Size) -> Self {
        self.fixed_size = Some(WidgetStateProperty::all(size));
        self
    }

    /// The same, at most `size`.
    #[must_use]
    pub fn maximum_size(mut self, size: Size) -> Self {
        self.maximum_size = Some(WidgetStateProperty::all(size));
        self
    }

    /// The same, its outline `property`.
    #[must_use]
    pub fn side(mut self, property: WidgetStateProperty<BorderSide>) -> Self {
        self.side = Some(property);
        self
    }

    /// The same, shaped `shape` in every state.
    #[must_use]
    pub fn shape(mut self, shape: ShapeBorder) -> Self {
        self.shape = Some(WidgetStateProperty::all(shape));
        self
    }

    /// The same, its icon `property`.
    #[must_use]
    pub fn icon_color(mut self, property: WidgetStateProperty<Color>) -> Self {
        self.icon_color = Some(property);
        self
    }

    /// The same, its icon `size` on a side.
    #[must_use]
    pub fn icon_size(mut self, size: f32) -> Self {
        self.icon_size = Some(WidgetStateProperty::all(size));
        self
    }

    /// The same, its label in `style`.
    #[must_use]
    pub fn text_style(mut self, style: TextStyle) -> Self {
        self.text_style = Some(WidgetStateProperty::all(style));
        self
    }

    /// The same, at `density`.
    #[must_use]
    pub fn visual_density(mut self, density: crate::VisualDensity) -> Self {
        self.visual_density = Some(density);
        self
    }

    /// The same, reserving `target` for a finger.
    #[must_use]
    pub fn tap_target(mut self, target: TapTarget) -> Self {
        self.tap_target = Some(target);
        self
    }

    /// The same, its content at `alignment`.
    #[must_use]
    pub fn alignment(mut self, alignment: Alignment) -> Self {
        self.alignment = Some(alignment);
        self
    }

    /// The same, its icon on the `alignment` side.
    #[must_use]
    pub fn icon_alignment(mut self, alignment: IconAlignment) -> Self {
        self.icon_alignment = Some(alignment);
        self
    }
}

/// `value` while enabled, and nothing while disabled: what a caller's plain colour or
/// number means, so that a disabled button keeps the disabled look (the reference's
/// `styleFrom`).
fn enabled_only<T>(value: T) -> WidgetStateProperty<T> {
    WidgetStateProperty::new().when(!crate::StateFilter::from(WidgetState::Disabled), value)
}

/// Puts `value` in `slot` if nothing is there yet.
fn fill<T>(slot: &mut Option<T>, value: Option<T>) {
    if slot.is_none() {
        *slot = value;
    }
}

/// The reference's `scaledPadding` (`button_style_button.dart:294`): the padding at the
/// standard text size, at twice it, at three times it, and in between.
fn scaled_padding(one: Insets, two: Insets, three: Insets, scale: f32) -> Insets {
    let lerp = |a: Insets, b: Insets, t: f32| {
        Insets::new(
            a.top + (b.top - a.top) * t,
            a.right + (b.right - a.right) * t,
            a.bottom + (b.bottom - a.bottom) * t,
            a.left + (b.left - a.left) * t,
        )
    };
    if scale <= 1.0 {
        one
    } else if scale < 2.0 {
        lerp(one, two, scale - 1.0)
    } else if scale < 3.0 {
        lerp(two, three, scale - 2.0)
    } else {
        three
    }
}

/// Padding given start and end, laid out for the reading direction.
fn directional(start: f32, top: f32, end: f32, bottom: f32, rtl: bool) -> Insets {
    if rtl {
        Insets::new(top, start, bottom, end)
    } else {
        Insets::new(top, end, bottom, start)
    }
}

/// **The reference's defaults for each kind of button**, Material 3
/// (`elevated_button.dart:513`, `filled_button.dart:531` and `:672`,
/// `outlined_button.dart:460`, `text_button.dart:493`), with the padding for a button that
/// has an icon (`elevated_button.dart:421`, `text_button.dart:412`).
fn default_style(variant: Variant, theme: &Theme, with_icon: bool) -> ButtonStyle {
    let c = &theme.scheme;
    let disabled = WidgetState::Disabled;
    // The reference's `onSurface` at 12 % and 38 %, resolved over the surface rather than
    // drawn translucent: the GPU blends in linear light, where 12 % paints like a third
    // (see `disabled.rs`).
    let disabled_fill = crate::disabled::disabled_container(theme);
    let disabled_ink = crate::disabled::disabled_content(theme);
    let (background, foreground, overlay): (Option<Color>, Color, Color) = match variant {
        Variant::Elevated => (Some(c.surface_container_low), c.primary, c.primary),
        Variant::Filled => (Some(c.primary), c.on_primary, c.on_primary),
        Variant::Tonal => (
            Some(c.secondary_container),
            c.on_secondary_container,
            c.on_secondary_container,
        ),
        Variant::Danger => (Some(c.error), c.on_error, c.on_error),
        Variant::Outlined | Variant::Text => (None, c.primary, c.primary),
    };
    // Raised at rest, higher under a pointer; flat and raised a step under a pointer; or
    // flat throughout.
    let (rest, hovered) = match variant {
        Variant::Elevated => (1.0, 3.0),
        Variant::Filled | Variant::Tonal | Variant::Danger => (0.0, 1.0),
        Variant::Outlined | Variant::Text => (0.0, 0.0),
    };
    let scale = text_size_scale(theme);
    let rtl = theme.direction == frus_core::TextDirection::Rtl;
    let padding = match (variant, with_icon) {
        (Variant::Text, false) => scaled_padding(
            Insets::new(8.0, 12.0, 8.0, 12.0),
            Insets::new(0.0, 8.0, 0.0, 8.0),
            Insets::new(0.0, 4.0, 0.0, 4.0),
            scale,
        ),
        (Variant::Text, true) => scaled_padding(
            directional(12.0, 8.0, 16.0, 8.0, rtl),
            Insets::new(0.0, 4.0, 0.0, 4.0),
            Insets::new(0.0, 4.0, 0.0, 4.0),
            scale,
        ),
        (_, false) => scaled_padding(
            Insets::new(0.0, BUTTON_PADDING, 0.0, BUTTON_PADDING),
            Insets::new(0.0, 12.0, 0.0, 12.0),
            Insets::new(0.0, 6.0, 0.0, 6.0),
            scale,
        ),
        (_, true) => scaled_padding(
            directional(16.0, 0.0, 24.0, 0.0, rtl),
            directional(8.0, 0.0, 12.0, 0.0, rtl),
            directional(4.0, 0.0, 6.0, 0.0, rtl),
            scale,
        ),
    };
    let mut style = ButtonStyle {
        text_style: Some(WidgetStateProperty::all(theme.text.label_large)),
        background_color: background.map(|bg| {
            WidgetStateProperty::new()
                .when(disabled, disabled_fill)
                .otherwise(bg)
        }),
        foreground_color: Some(
            WidgetStateProperty::new()
                .when(disabled, disabled_ink)
                .otherwise(foreground),
        ),
        // The ripple of a press, the highlight under a pointer, and the keyboard's focus
        // (`elevated_button.dart:547`): ten, eight and ten percent.
        overlay_color: Some(
            WidgetStateProperty::new()
                .when(WidgetState::Pressed, overlay.with_alpha(0.1))
                .when(WidgetState::Hovered, overlay.with_alpha(0.08))
                .when(WidgetState::Focused, overlay.with_alpha(0.1)),
        ),
        shadow_color: Some(WidgetStateProperty::all(c.shadow)),
        elevation: Some(
            WidgetStateProperty::new()
                .when(disabled, 0.0)
                .when(WidgetState::Pressed, rest)
                .when(WidgetState::Hovered, hovered)
                .otherwise(rest),
        ),
        padding: Some(WidgetStateProperty::all(padding)),
        minimum_size: Some(WidgetStateProperty::all(Size::new(
            BUTTON_MIN_WIDTH,
            BUTTON_HEIGHT,
        ))),
        fixed_size: None,
        maximum_size: Some(WidgetStateProperty::all(Size::new(
            f32::INFINITY,
            f32::INFINITY,
        ))),
        icon_color: None,
        icon_size: Some(WidgetStateProperty::all(BUTTON_ICON_SIZE)),
        side: None,
        shape: Some(WidgetStateProperty::all(ShapeBorder::stadium())),
        visual_density: Some(theme.visual_density()),
        tap_target: Some(theme.tap_target),
        alignment: Some(Alignment::CENTER),
        icon_alignment: Some(IconAlignment::Start),
    };
    if variant == Variant::Outlined {
        // The outline: faint while disabled, the accent while focused, the outline role
        // otherwise (`outlined_button.dart:553`).
        style.side = Some(
            WidgetStateProperty::new()
                .when(
                    disabled,
                    BorderSide::new(disabled_fill, BUTTON_BORDER_WIDTH),
                )
                .when(
                    WidgetState::Focused,
                    BorderSide::new(c.primary, BUTTON_BORDER_WIDTH),
                )
                .otherwise(BorderSide::new(c.outline, BUTTON_BORDER_WIDTH)),
        );
    }
    style
}

/// How much larger than the standard the reader's label is: the label's size at the
/// reader's text scale over 14 (`elevated_button.dart:460`).
fn text_size_scale(theme: &Theme) -> f32 {
    let size = theme.text.label_large.size.unwrap_or(14.0);
    size * frus_core::text_scale() / 14.0
}

/// Resolves one property for `states`: the button's, the theme's, the default's.
fn resolve<T: Clone>(
    states: WidgetStates,
    get: impl Fn(&ButtonStyle) -> Option<&WidgetStateProperty<T>>,
    styles: [Option<&ButtonStyle>; 3],
) -> Option<T> {
    styles
        .into_iter()
        .flatten()
        .find_map(|style| get(style).and_then(|p| p.resolve(states)).cloned())
}

/// Resolves one state-independent setting: the button's, the theme's, the default's.
fn setting<T: Copy>(
    get: impl Fn(&ButtonStyle) -> Option<T>,
    styles: [Option<&ButtonStyle>; 3],
) -> Option<T> {
    styles.into_iter().flatten().find_map(get)
}

/// The content a button was given, kept until the walk first asks for its children: the
/// icon beside it is put together then, once and for all.
struct Parts<Msg> {
    content: Box<dyn Widget<Msg>>,
    icon: Option<Box<dyn Widget<Msg>>>,
}

/// A clickable button: the reference's `ElevatedButton`, `FilledButton`,
/// `FilledButton.tonal`, `OutlinedButton` and `TextButton`, chosen by [`Variant`].
pub struct Button<Msg = crate::callback::Callback> {
    label: Option<String>,
    parts: std::cell::RefCell<Option<Parts<Msg>>>,
    built: std::cell::OnceCell<Vec<Box<dyn Widget<Msg>>>>,
    has_icon: bool,
    variant: Variant,
    on_press: Option<Msg>,
    on_long_press: Option<Msg>,
    /// `false` turns the button off whatever its actions.
    enabled: bool,
    style: ButtonStyle,
    /// The older builders' word on the minimum width and the fixed height, which name one
    /// side of a size the style holds whole.
    min_width: Option<f32>,
    height: Option<f32>,
}

impl<Msg: Clone + 'static> Button<Msg> {
    /// A button reading `label`.
    pub fn new(label: impl Into<String>) -> Self {
        let label = label.into();
        // The button's own node says the label; the words drawn inside it say nothing,
        // so a reader hears it once.
        let mut button = Self::with_child(crate::ExcludeSemantics::new(crate::Text::new(
            label.clone(),
        )));
        button.label = Some(label);
        button
    }

    /// A button holding `child` — any widget, as the reference's buttons hold any widget.
    /// The child's text takes the button's label style and colour, and its icons the
    /// button's icon colour and size.
    pub fn with_child(child: impl Widget<Msg> + 'static) -> Self {
        Self {
            label: None,
            parts: std::cell::RefCell::new(Some(Parts {
                content: Box::new(child),
                icon: None,
            })),
            built: std::cell::OnceCell::new(),
            has_icon: false,
            variant: Variant::default(),
            on_press: None,
            on_long_press: None,
            enabled: true,
            style: ButtonStyle::default(),
            min_width: None,
            height: None,
        }
    }

    /// **An icon beside the content** — the reference's `.icon` constructors: before the
    /// label unless [`icon_alignment`](Self::icon_alignment) says after, 8 px from it
    /// (4 at twice the standard text size), and the padding becomes the reference's for
    /// a button with an icon.
    pub fn icon(mut self, icon: impl Widget<Msg> + 'static) -> Self {
        if let Some(parts) = self.parts.get_mut().as_mut() {
            parts.icon = Some(Box::new(icon));
            self.has_icon = true;
        }
        self
    }
}

impl<Msg> Button<Msg> {
    /// Enables or **disables** the button. A button is also disabled when it has neither
    /// a press nor a long press to answer, as the reference's is.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Is it on: told so, and with something to do?
    fn is_enabled(&self) -> bool {
        self.enabled && (self.on_press.is_some() || self.on_long_press.is_some())
    }

    /// **How it looks, state by state** — over the theme's style and the reference's
    /// defaults. The builders below each set one property of it.
    #[must_use]
    pub fn style(mut self, style: ButtonStyle) -> Self {
        self.style = style;
        self
    }

    /// The corner radii (uniform via `f32`, per corner via [`BorderRadius`]). Unset, a
    /// **stadium**.
    pub fn radius(mut self, radius: impl Into<BorderRadius>) -> Self {
        self.style.shape = Some(WidgetStateProperty::all(ShapeBorder::rounded(radius)));
        self
    }

    /// **What shape it is** (`shape_border.dart`).
    #[must_use]
    pub fn shape(mut self, shape: ShapeBorder) -> Self {
        self.style.shape = Some(WidgetStateProperty::all(shape));
        self
    }

    /// Which of the reference's buttons it is.
    pub fn variant(mut self, variant: Variant) -> Self {
        self.variant = variant;
        self
    }

    /// The label's type. Unset, the theme's `label_large`.
    pub fn label_style(mut self, style: TextStyle) -> Self {
        self.style.text_style = Some(WidgetStateProperty::all(style));
        self
    }

    /// Font size — sugar for [`Button::label_style`] with the label step resized.
    pub fn size(mut self, size: f32) -> Self {
        self.style.text_style = Some(WidgetStateProperty::all(TextStyle::new(size)));
        self
    }

    /// The surface while enabled.
    pub fn color(mut self, color: Color) -> Self {
        self.style.background_color = Some(enabled_only(color));
        self
    }

    /// The label's colour while enabled.
    pub fn label_color(mut self, color: Color) -> Self {
        self.style.foreground_color = Some(enabled_only(color));
        self
    }

    /// The outline's colour, `1` px thick unless [`border_width`](Self::border_width) says.
    pub fn border_color(mut self, color: Color) -> Self {
        let width = self.border_width_said().unwrap_or(BUTTON_BORDER_WIDTH);
        self.style.side = Some(enabled_only(BorderSide::new(color, width)));
        self
    }

    /// The outline's thickness; `0.0` removes it.
    pub fn border_width(mut self, width: f32) -> Self {
        let color = self
            .style
            .side
            .as_ref()
            .and_then(|s| s.resolve(WidgetStates::EMPTY))
            .map(|s| s.color);
        self.style.side = Some(match color {
            Some(color) => enabled_only(BorderSide::new(color, width)),
            None => WidgetStateProperty::all(BorderSide::new(Color::TRANSPARENT, width)),
        });
        self
    }

    fn border_width_said(&self) -> Option<f32> {
        self.style
            .side
            .as_ref()
            .and_then(|s| s.resolve(WidgetStates::EMPTY))
            .map(|s| s.width)
    }

    /// The room either side of the content.
    pub fn padding(mut self, padding: f32) -> Self {
        self.style.padding = Some(WidgetStateProperty::all(Insets::new(
            0.0, padding, 0.0, padding,
        )));
        self
    }

    /// **The button's height**, exactly: the reference's fixed height, which the visual
    /// density does not move.
    pub fn height(mut self, height: f32) -> Self {
        self.height = Some(height);
        self
    }

    /// The narrowest it will be, before the visual density.
    pub fn min_width(mut self, width: f32) -> Self {
        self.min_width = Some(width);
        self
    }

    /// How far it sits off the surface **in every state** while enabled. Unset, the
    /// variant's: an elevated button rests at 1 and rises to 3 under a pointer; a filled
    /// or tonal one is flat and rises to 1; an outlined or text one stays flat.
    pub fn elevation(mut self, elevation: f32) -> Self {
        self.style.elevation = Some(enabled_only(elevation));
        self
    }

    /// Which side of the label an icon goes.
    #[must_use]
    pub fn icon_alignment(mut self, alignment: IconAlignment) -> Self {
        self.style.icon_alignment = Some(alignment);
        self
    }

    /// Message emitted on a press.
    pub fn on_press(mut self, message: impl Into<Msg>) -> Self {
        self.on_press = Some(message.into());
        self
    }

    /// Message emitted on a **long press**, which takes the place of the press.
    #[must_use]
    pub fn on_long_press(mut self, message: impl Into<Msg>) -> Self {
        self.on_long_press = Some(message.into());
        self
    }

    /// The theme's style for this kind of button: its own style for the kind, with the
    /// theme's plain fields under it.
    fn theme_style(&self, theme: &Theme) -> ButtonStyle {
        themed_style(theme, self.variant)
    }
}

/// The theme's style for `variant`: the style it gives that kind of button, with its plain
/// fields under it.
fn themed_style(theme: &Theme, variant: Variant) -> ButtonStyle {
    let t = &theme.widgets.button;
    let mut style = match variant {
        Variant::Elevated => t.elevated_style.clone(),
        Variant::Filled | Variant::Tonal | Variant::Danger => t.filled_style.clone(),
        Variant::Outlined => t.outlined_style.clone(),
        Variant::Text => t.text_button_style.clone(),
    }
    .unwrap_or_default();
    fill(&mut style.background_color, t.color.map(enabled_only));
    fill(&mut style.foreground_color, t.label_color.map(enabled_only));
    fill(
        &mut style.text_style,
        t.label_style.map(WidgetStateProperty::all),
    );
    fill(
        &mut style.shape,
        t.shape
            .or(t.radius.map(ShapeBorder::rounded))
            .map(WidgetStateProperty::all),
    );
    fill(
        &mut style.padding,
        t.padding
            .map(|p| WidgetStateProperty::all(Insets::new(0.0, p, 0.0, p))),
    );
    fill(&mut style.elevation, t.elevation.map(enabled_only));
    if style.side.is_none() && (t.border_color.is_some() || t.border_width.is_some()) {
        style.side = Some(enabled_only(BorderSide::new(
            t.border_color.unwrap_or(theme.scheme.outline),
            t.border_width.unwrap_or(BUTTON_BORDER_WIDTH),
        )));
    }
    style
}

impl<Msg> Button<Msg> {
    /// The states it is in, disabled included.
    fn states(&self, status: &Status) -> WidgetStates {
        if self.is_enabled() {
            status.states()
        } else {
            WidgetStates::of(WidgetState::Disabled)
        }
    }

    /// Everything about it, resolved for `states`.
    fn resolved(&self, theme: &Theme, states: WidgetStates) -> Resolved {
        let own = &self.style;
        let themed = self.theme_style(theme);
        let default = default_style(self.variant, theme, self.has_icon);
        let styles = [Some(own), Some(&themed), Some(&default)];
        let r =
            |f: fn(&ButtonStyle) -> Option<&WidgetStateProperty<Color>>| resolve(states, f, styles);
        let foreground = r(|s| s.foreground_color.as_ref()).unwrap_or(theme.scheme.primary);
        // The icon's colour: the icon colour said by the button or the theme, then their
        // foreground, then the default's icon colour, then its foreground
        // (`button_style_button.dart:395`).
        let icon_color = resolve(
            states,
            |s| s.icon_color.as_ref(),
            [Some(own), Some(&themed), None],
        )
        .or_else(|| {
            resolve(
                states,
                |s| s.foreground_color.as_ref(),
                [Some(own), Some(&themed), None],
            )
        })
        .or_else(|| {
            resolve(
                states,
                |s| s.icon_color.as_ref(),
                [Some(&default), None, None],
            )
        })
        .unwrap_or(foreground);
        let mut minimum = resolve(states, |s| s.minimum_size.as_ref(), styles)
            .unwrap_or(Size::new(BUTTON_MIN_WIDTH, BUTTON_HEIGHT));
        if let Some(w) = self.min_width {
            minimum.width = w;
        }

        Resolved {
            text_style: resolve(states, |s| s.text_style.as_ref(), styles)
                .unwrap_or(theme.text.label_large),
            background: r(|s| s.background_color.as_ref()),
            foreground,
            overlay: r(|s| s.overlay_color.as_ref()),
            shadow: r(|s| s.shadow_color.as_ref()).unwrap_or(theme.scheme.shadow),
            elevation: resolve(states, |s| s.elevation.as_ref(), styles).unwrap_or(0.0),
            padding: resolve(states, |s| s.padding.as_ref(), styles).unwrap_or(Insets::ZERO),
            minimum,
            fixed: match (
                resolve(states, |s| s.fixed_size.as_ref(), styles),
                self.height,
            ) {
                (fixed, None) => fixed,
                (Some(size), Some(h)) => Some(Size::new(size.width, h)),
                (None, Some(h)) => Some(Size::new(f32::INFINITY, h)),
            },
            maximum: resolve(states, |s| s.maximum_size.as_ref(), styles)
                .unwrap_or(Size::new(f32::INFINITY, f32::INFINITY)),
            icon_color,
            icon_size: resolve(states, |s| s.icon_size.as_ref(), styles)
                .unwrap_or(BUTTON_ICON_SIZE),
            side: resolve(states, |s| s.side.as_ref(), styles),
            shape: resolve(states, |s| s.shape.as_ref(), styles).unwrap_or(ShapeBorder::stadium()),
            density: setting(|s| s.visual_density, styles).unwrap_or(theme.visual_density()),
            tap_target: setting(|s| s.tap_target, styles).unwrap_or(theme.tap_target),
            alignment: setting(|s| s.alignment, styles).unwrap_or(Alignment::CENTER),
        }
    }

    /// The box laid out and the button drawn inside it: the room the touch target adds
    /// around the visible button on each axis, and the visible button's minimum size
    /// (`button_style_button.dart:473`, `:578`).
    fn geometry(&self, r: &Resolved) -> Geometry {
        let (dx, dy) = r.density.base_size_adjustment();
        let (mut min_w, mut min_h) = r.density.effective_min_size(
            (r.minimum.width, r.minimum.height),
            (r.maximum.width, r.maximum.height),
        );
        let mut fixed = (None, None);
        if let Some(size) = r.fixed {
            if size.width.is_finite() {
                let w = size.width.clamp(min_w, r.maximum.width.max(min_w));
                min_w = w;
                fixed.0 = Some(w);
            }
            if size.height.is_finite() {
                let h = size.height.clamp(min_h, r.maximum.height.max(min_h));
                min_h = h;
                fixed.1 = Some(h);
            }
        }
        let tap = match r.tap_target {
            TapTarget::Padded => (crate::MIN_TAP_TARGET + dx, crate::MIN_TAP_TARGET + dy),
            TapTarget::ShrinkWrap => (0.0, 0.0),
        };
        let extra = (
            ((tap.0 - min_w) * 0.5).max(0.0),
            ((tap.1 - min_h) * 0.5).max(0.0),
        );
        // The density moves the padding too: across never inwards, down either way, and
        // never below nothing (`button_style_button.dart:501`).
        let p = r.padding;
        let ddx = dx.max(0.0);
        let padding = Insets::new(
            (p.top + dy).max(0.0),
            (p.right + ddx).max(0.0),
            (p.bottom + dy).max(0.0),
            (p.left + ddx).max(0.0),
        );
        Geometry {
            extra,
            min: (min_w, min_h),
            fixed,
            padding,
        }
    }
}

/// A button's style, resolved for one set of states.
struct Resolved {
    text_style: TextStyle,
    background: Option<Color>,
    foreground: Color,
    overlay: Option<Color>,
    shadow: Color,
    elevation: f32,
    padding: Insets,
    minimum: Size,
    fixed: Option<Size>,
    maximum: Size,
    icon_color: Color,
    icon_size: f32,
    side: Option<BorderSide>,
    shape: ShapeBorder,
    density: crate::VisualDensity,
    tap_target: TapTarget,
    alignment: Alignment,
}

/// Where a button's parts go.
struct Geometry {
    /// The touch target's room around the visible button, across and down, on each side.
    extra: (f32, f32),
    /// The visible button's minimum size.
    min: (f32, f32),
    /// Its fixed width and height, where it has them.
    fixed: (Option<f32>, Option<f32>),
    /// The room between its edge and its content.
    padding: Insets,
}

/// `top` over `base` at `amount` of its own opacity.
fn over(base: Color, top: Color, amount: f32) -> Color {
    let a = (top.a * amount).clamp(0.0, 1.0);
    if a <= 0.0 {
        return base;
    }
    if base.a <= 0.0 {
        return top.with_alpha(a);
    }
    base.lerp(top.with_alpha(base.a), a)
}

/// Where along an axis an alignment puts the content.
fn place(v: f32) -> (Justify, Align) {
    let j = if v < -0.5 {
        Justify::Start
    } else if v > 0.5 {
        Justify::End
    } else {
        Justify::Center
    };
    let a = match j {
        Justify::Start => Align::Start,
        Justify::End => Align::End,
        _ => Align::Center,
    };
    (j, a)
}

/// An icon beside a label, as the reference's `.icon` buttons lay them out
/// (`elevated_button.dart:470`): 8 px apart at the standard text size, closing to 4 at
/// twice it.
struct IconLabel<Msg> {
    children: Vec<Box<dyn Widget<Msg>>>,
    /// The button's own word on the icon's side.
    own: Option<IconAlignment>,
    /// Which kind of button, for the theme's word.
    variant: Variant,
}

impl<Msg> Widget<Msg> for IconLabel<Msg> {
    fn style(&self) -> Style {
        self.style_themed(&Theme::default())
    }

    fn style_themed(&self, theme: &Theme) -> Style {
        let scale = (text_size_scale(theme).clamp(1.0, 2.0)) - 1.0;
        Style {
            flex_direction: if self
                .own
                .or_else(|| themed_style(theme, self.variant).icon_alignment)
                .unwrap_or_default()
                == IconAlignment::End
            {
                FlexDirection::RowReverse
            } else {
                FlexDirection::Row
            },
            align: Align::Center,
            gap: 8.0 + (4.0 - 8.0) * scale,
            ..Default::default()
        }
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &self.children
    }

    fn paint(&self, _bounds: Rect, _status: Status, _theme: &Theme, _scene: &mut Scene) {}

    fn on_click(&self) -> Option<Msg> {
        None
    }
}

impl<Msg: Clone + 'static> Widget<Msg> for Button<Msg> {
    fn style(&self) -> Style {
        Widget::<Msg>::style_themed(self, &Theme::default())
    }

    fn style_themed(&self, theme: &Theme) -> Style {
        let r = self.resolved(theme, self.states(&Status::default()));
        let g = self.geometry(&r);
        let (justify, _) = place(r.alignment.x);
        let (_, align) = place(r.alignment.y);
        // A labelled button knows its width before it is laid out — the label, the
        // padding, the icon and its gap, within the minimum and the maximum — and says it,
        // so that a bar deciding which actions fit can ask. Any other content is measured
        // by the layout.
        let natural = self.label.as_ref().map(|label| {
            let text = frus_text::measure_style(label, r.text_style).width;
            let icon = if self.has_icon {
                let scale = text_size_scale(theme).clamp(1.0, 2.0) - 1.0;
                r.icon_size + 8.0 + (4.0 - 8.0) * scale
            } else {
                0.0
            };
            (text + icon + g.padding.left + g.padding.right)
                .ceil()
                .clamp(g.min.0, r.maximum.width.max(g.min.0))
        });
        let width = match (g.fixed.0, natural) {
            (Some(w), _) | (None, Some(w)) => Dimension::Length(w + 2.0 * g.extra.0),
            (None, None) => Dimension::Auto,
        };
        let height = g
            .fixed
            .1
            .map_or(Dimension::Auto, |h| Dimension::Length(h + 2.0 * g.extra.1));
        let length = |v: f32| {
            if v.is_finite() {
                Dimension::Length(v)
            } else {
                Dimension::Auto
            }
        };
        Style {
            width,
            height,
            // A width already said holds the minimum. Saying both, equal, made the layout
            // measure the label at no width at all: "Edit" one letter a line, and a
            // button 85 px tall inside a 56 px bar.
            min_width: if matches!(width, Dimension::Length(_)) {
                Dimension::Auto
            } else {
                Dimension::Length(g.min.0 + 2.0 * g.extra.0)
            },
            min_height: Dimension::Length(g.min.1 + 2.0 * g.extra.1),
            max_width: length(r.maximum.width + 2.0 * g.extra.0),
            max_height: length(r.maximum.height + 2.0 * g.extra.1),
            flex_direction: FlexDirection::Row,
            justify,
            align,
            padding: Insets::new(
                g.padding.top + g.extra.1,
                g.padding.right + g.extra.0,
                g.padding.bottom + g.extra.1,
                g.padding.left + g.extra.0,
            ),
            ..Default::default()
        }
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        self.built.get_or_init(|| {
            let Some(Parts { content, icon }) = self.parts.borrow_mut().take() else {
                return Vec::new();
            };
            let node: Box<dyn Widget<Msg>> = match icon {
                None => content,
                Some(icon) => Box::new(IconLabel {
                    children: vec![icon, content],
                    own: self.style.icon_alignment,
                    variant: self.variant,
                }),
            };
            vec![node]
        })
    }

    /// The content's text takes the button's label style in its foreground colour, and
    /// its icons the button's icon colour and size, as the reference hands them down
    /// through its `Material` and its `IconTheme` (`button_style_button.dart:550`, `:601`).
    fn theme_override(&self, inherited: &Theme) -> Option<Box<Theme>> {
        let r = self.resolved(inherited, self.states(&Status::default()));
        let mut theme = Box::new(inherited.clone());
        // In place of the text style around it, not merged into it: inside an app bar the
        // style around is the title's, and a label that took its spacing would no longer
        // be the width the button measured (the reference's `Material` sets its text style
        // outright).
        theme.widgets.text =
            crate::DefaultTextStyle::from_text_style(r.text_style.color(r.foreground));
        theme.widgets.icon.color = Some(r.icon_color);
        theme.widgets.icon.size = Some(r.icon_size);
        Some(theme)
    }

    fn paint(&self, bounds: Rect, status: Status, theme: &Theme, scene: &mut Scene) {
        let o = status.opacity;
        let states = self.states(&status);
        let base = if self.is_enabled() {
            WidgetStates::EMPTY
        } else {
            WidgetStates::of(WidgetState::Disabled)
        };
        let r = self.resolved(theme, states);
        let g = self.geometry(&r);
        let visible = Rect::new(
            bounds.x + g.extra.0,
            bounds.y + g.extra.1,
            (bounds.width - 2.0 * g.extra.0).max(0.0),
            (bounds.height - 2.0 * g.extra.1).max(0.0),
        );
        let radius = r
            .shape
            .as_rounded(visible)
            .map(|(_, radius)| radius)
            .unwrap_or(BorderRadius::ZERO);

        // The elevation in each state, eased between by the same progress the highlight
        // fades by: a press outranks a hover, a hover a focus.
        let at = |extra: Option<WidgetState>| {
            let s = extra.map_or(base, |e| base.set(e, true));
            self.resolved(theme, s).elevation
        };
        let elevation = if self.is_enabled() {
            let toward = |from: f32, to: f32, p: f32| from + (to - from) * p.clamp(0.0, 1.0);
            let e = toward(
                at(None),
                at(Some(WidgetState::Focused)),
                status.focus_progress,
            );
            let e = toward(e, at(Some(WidgetState::Hovered)), status.hover_progress);
            toward(e, at(Some(WidgetState::Pressed)), status.press_progress)
        } else {
            r.elevation
        };
        if elevation > 0.0 {
            frus_core::paint_elevation(scene, visible, radius, elevation, r.shadow.fade(o));
        }

        // The surface, with the focus and hover highlights over it, each fading in by its
        // own progress (`ink_well.dart` highlights). The press is the ripple's, not a
        // highlight: its overlay colour is the splash (see `ink`).
        let mut fill = r.background.unwrap_or(Color::TRANSPARENT);
        if self.is_enabled() {
            let overlay = |s: WidgetState| self.resolved(theme, base.set(s, true)).overlay;
            if let Some(focus) = overlay(WidgetState::Focused) {
                fill = over(fill, focus, status.focus_progress);
            }
            if let Some(hover) = overlay(WidgetState::Hovered) {
                fill = over(fill, hover, status.hover_progress);
            }
        }
        let shape = match r.side {
            Some(side) => r
                .shape
                .with_side(BorderSide::new(side.color.fade(o), side.width)),
            None => r.shape,
        };
        scene.draw_shape(visible, shape, fill.fade(o));
    }

    fn on_click(&self) -> Option<Msg> {
        if self.is_enabled() {
            self.on_press.clone()
        } else {
            None
        }
    }

    fn on_long_press(&self) -> Option<Msg> {
        if self.is_enabled() {
            self.on_long_press.clone()
        } else {
            None
        }
    }

    fn ink(&self, theme: &Theme) -> Option<crate::InkStyle> {
        // A disabled control does not answer a tap, so it does not splash either.
        if !self.is_enabled() {
            return None;
        }
        // The ripple is the pressed overlay (`button_style_button.dart:566`), unless the
        // application has named one for every surface.
        let pressed = self.resolved(theme, WidgetStates::of(WidgetState::Pressed));
        let splash = theme
            .widgets
            .ink
            .color
            .or(pressed.overlay)
            .unwrap_or_else(|| pressed.foreground.with_alpha(0.1));
        let g = self.geometry(&pressed);
        let side = g.min.1;
        Some(
            crate::InkStyle::of(theme).color(splash).radius(
                pressed
                    .shape
                    .as_rounded(Rect::new(0.0, 0.0, side, side))
                    .map(|(_, radius)| radius)
                    .unwrap_or(BorderRadius::ZERO),
            ),
        )
    }

    fn focusable(&self) -> bool {
        self.is_enabled()
    }

    fn semantics(&self) -> Option<frus_core::SemanticsProperties> {
        // A labelled button's node carries its label, as the reference's button takes in
        // the text inside it (`button_style_button.dart:591`).
        let mut semantics = frus_core::SemanticsProperties::new(frus_core::Role::Button);
        if let Some(label) = &self.label {
            semantics = semantics.label(label.clone());
        }
        // A disabled button does not announce a clickable action.
        Some(if self.is_enabled() {
            semantics.clickable()
        } else {
            semantics.disabled(true)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{build_ui, Flex, Runtime, VisualDensity};
    use frus_core::{Primitive, TargetPlatform};

    #[derive(Clone, Debug, PartialEq)]
    enum Msg {
        Pressed,
        Held,
    }

    /// A phone's theme: the standard density, so a button is the reference's 64 × 40 in a
    /// 48 px touch target.
    fn phone() -> Theme {
        Theme::dark().with_platform(TargetPlatform::Android)
    }

    /// The button laid out alone at the top left of a large surface: the box it takes
    /// and what it paints.
    fn laid_out(button: Button<Msg>, theme: &Theme) -> (Rect, Vec<Primitive>) {
        let tree = Flex::<Msg>::column()
            .width(400.0)
            .height(300.0)
            .align(frus_layout::Align::Start)
            .child(button);
        let ui = build_ui(
            &tree as &dyn Widget<Msg>,
            Size::new(400.0, 300.0),
            &Runtime::default(),
            theme,
        );
        let prims = ui.scene().primitives().to_vec();
        let surface = prims
            .iter()
            .find_map(|p| match p {
                Primitive::Rect { rect, blur, .. } if *blur == 0.0 => Some(*rect),
                _ => None,
            })
            .unwrap_or(Rect::new(0.0, 0.0, 0.0, 0.0));
        (surface, prims)
    }

    fn label_colour(prims: &[Primitive]) -> Option<Color> {
        prims.iter().find_map(|p| match p {
            Primitive::Text { color, .. } => Some(*color),
            _ => None,
        })
    }

    fn surface_colour(prims: &[Primitive]) -> Option<Color> {
        prims.iter().find_map(|p| match p {
            Primitive::Rect { color, blur, .. } if *blur == 0.0 => Some(*color),
            _ => None,
        })
    }

    /// **The reference's size**: at least 64 × 40, centred in a 48 px touch target, and
    /// as wide as its label and 24 px either side (`elevated_button.dart:593`, `:458`;
    /// `button_style_button.dart:578`).
    #[test]
    fn a_button_is_the_reference_s_size() {
        // Nothing but padding: 48 px, under the minimum.
        let (short, _) = laid_out(Button::new("").on_press(Msg::Pressed), &phone());
        assert_eq!((short.width, short.height), (64.0, 40.0), "the minimum");
        assert_eq!(short.y, 4.0, "centred in a 48 px target");
        let (long, _) = laid_out(
            Button::new("A rather longer label").on_press(Msg::Pressed),
            &phone(),
        );
        let text = frus_text::measure_style("A rather longer label", phone().text.label_large);
        assert!(
            (long.width - (text.width + 48.0)).abs() < 1.0,
            "the label and 24 px either side: {} vs {}",
            long.width,
            text.width
        );
        // A text button keeps 12 px either side and 8 above and below.
        let (text_button, _) = laid_out(
            Button::new("A rather longer label")
                .variant(Variant::Text)
                .on_press(Msg::Pressed),
            &phone(),
        );
        assert!((text_button.width - (text.width + 24.0)).abs() < 1.0);
    }

    /// **The desktops are compact** (milestone 631): the minimum loses 8 px each way and
    /// so does the touch target, so the button is 56 × 32 in a 40 px target.
    #[test]
    fn a_desktop_s_button_is_compact() {
        let desk = Theme::dark().with_platform(TargetPlatform::Windows);
        let (b, _) = laid_out(Button::new("").on_press(Msg::Pressed), &desk);
        assert_eq!((b.width, b.height, b.y), (56.0, 32.0, 4.0));
        let told = desk.with_visual_density(VisualDensity::STANDARD);
        let (b, _) = laid_out(Button::new("").on_press(Msg::Pressed), &told);
        assert_eq!((b.width, b.height), (64.0, 40.0));
        // Shrink-wrapped, there is no room around it.
        let mut wrapped = phone();
        wrapped.tap_target = TapTarget::ShrinkWrap;
        let (b, _) = laid_out(Button::new("OK").on_press(Msg::Pressed), &wrapped);
        assert_eq!(b.y, 0.0);
    }

    /// **The padding shrinks as the reader's text grows** (`button_style_button.dart:294`):
    /// 24 px either side at the standard size, 12 at twice it, 6 at three times.
    #[test]
    fn the_padding_follows_the_text_size() {
        let p = |scale: f32| {
            frus_core::with_text_scale(scale, || {
                default_style(Variant::Filled, &phone(), false)
                    .padding
                    .unwrap()
                    .resolve(WidgetStates::EMPTY)
                    .copied()
                    .unwrap()
                    .left
            })
        };
        assert_eq!(p(1.0), 24.0);
        assert_eq!(p(1.5), 18.0);
        assert_eq!(p(2.0), 12.0);
        assert_eq!(p(3.0), 6.0);
        // With an icon: 16 before and 24 after, in the reading direction.
        let icon = default_style(Variant::Filled, &phone(), true)
            .padding
            .unwrap()
            .resolve(WidgetStates::EMPTY)
            .copied()
            .unwrap();
        assert_eq!((icon.left, icon.right), (16.0, 24.0));
        let rtl = default_style(Variant::Filled, &phone().rtl(), true)
            .padding
            .unwrap()
            .resolve(WidgetStates::EMPTY)
            .copied()
            .unwrap();
        assert_eq!((rtl.left, rtl.right), (24.0, 16.0));
    }

    /// **Each kind's colours are the reference's**, enabled and disabled; a button with
    /// nothing to do is disabled, as the reference's is.
    #[test]
    fn each_kind_has_the_reference_s_colours() {
        let t = phone();
        let c = &t.scheme;
        let colours = |v: Variant, on: bool| {
            let b = Button::new("OK").variant(v);
            let b = if on { b.on_press(Msg::Pressed) } else { b };
            let (_, prims) = laid_out(b, &t);
            (surface_colour(&prims), label_colour(&prims))
        };
        assert_eq!(
            colours(Variant::Filled, true),
            (Some(c.primary), Some(c.on_primary))
        );
        assert_eq!(
            colours(Variant::Tonal, true),
            (Some(c.secondary_container), Some(c.on_secondary_container))
        );
        assert_eq!(
            colours(Variant::Elevated, true),
            (Some(c.surface_container_low), Some(c.primary))
        );
        assert_eq!(colours(Variant::Text, true).1, Some(c.primary));
        // Nothing to do: disabled, on_surface at 12 % under on_surface at 38 %, resolved
        // over the surface.
        assert_eq!(
            colours(Variant::Filled, false),
            (
                Some(crate::disabled::disabled_container(&t)),
                Some(crate::disabled::disabled_content(&t))
            )
        );
        let off = Button::new("OK").variant(Variant::Filled);
        assert!(!Widget::<Msg>::focusable(&off));
        assert_eq!(Widget::<Msg>::on_click(&off), None);
        // A long press alone is something to do.
        let held = Button::new("OK").on_long_press(Msg::Held);
        assert!(Widget::<Msg>::focusable(&held));
        assert_eq!(Widget::<Msg>::on_long_press(&held), Some(Msg::Held));
    }

    /// **The overlays are the reference's**: 8 % under a pointer, 10 % focused, and the
    /// ripple of a press 10 % — in the label's colour for a filled button, the accent for
    /// the others (`elevated_button.dart:547`, `filled_button.dart:565`).
    #[test]
    fn the_overlays_are_the_reference_s() {
        let t = phone();
        let c = &t.scheme;
        let b = Button::new("OK").on_press(Msg::Pressed);
        let ink = Widget::<Msg>::ink(&b, &t).unwrap();
        assert_eq!(ink.color, c.on_primary.with_alpha(0.1));
        let e = Button::new("OK")
            .variant(Variant::Elevated)
            .on_press(Msg::Pressed);
        assert_eq!(
            Widget::<Msg>::ink(&e, &t).unwrap().color,
            c.primary.with_alpha(0.1)
        );
        let hover = e.resolved(&t, WidgetStates::of(WidgetState::Hovered));
        assert_eq!(hover.overlay, Some(c.primary.with_alpha(0.08)));
        let focus = e.resolved(&t, WidgetStates::of(WidgetState::Focused));
        assert_eq!(focus.overlay, Some(c.primary.with_alpha(0.1)));
        // The elevations: an elevated button rests at 1, rises to 3 under a pointer and
        // is back at 1 pressed; a filled one is flat and rises to 1.
        assert_eq!(hover.elevation, 3.0);
        assert_eq!(e.resolved(&t, WidgetStates::EMPTY).elevation, 1.0);
        assert_eq!(
            e.resolved(&t, WidgetState::Pressed | WidgetState::Hovered)
                .elevation,
            1.0
        );
        assert_eq!(
            b.resolved(&t, WidgetStates::of(WidgetState::Hovered))
                .elevation,
            1.0
        );
        assert_eq!(b.resolved(&t, WidgetStates::EMPTY).elevation, 0.0);
    }

    /// **An outlined button's outline** is the outline role, the accent while focused,
    /// and faint while disabled (`outlined_button.dart:553`).
    #[test]
    fn an_outline_follows_the_state() {
        let t = phone();
        let c = &t.scheme;
        let b = Button::<Msg>::new("OK")
            .variant(Variant::Outlined)
            .on_press(Msg::Pressed);
        let side = |s: WidgetStates| b.resolved(&t, s).side.unwrap().color;
        assert_eq!(side(WidgetStates::EMPTY), c.outline);
        assert_eq!(side(WidgetStates::of(WidgetState::Focused)), c.primary);
        assert_eq!(
            side(WidgetStates::of(WidgetState::Disabled)),
            crate::disabled::disabled_container(&t)
        );
    }

    /// **Any widget as content, an icon beside it**: the content's text takes the
    /// button's label colour and its icons the button's icon colour and 18 px
    /// (`button_style_button.dart:550`).
    #[test]
    fn any_content_takes_the_button_s_colours() {
        let t = phone();
        let b = Button::with_child(crate::Text::new("Mine"))
            .icon(crate::Icon::new(crate::Icons::ADD))
            .on_press(Msg::Pressed);
        let (surface, prims) = laid_out(b, &t);
        assert_eq!(label_colour(&prims), Some(t.scheme.on_primary));
        let glyph = prims.iter().find_map(|p| match p {
            Primitive::Path { fill: Some(c), .. } => Some(*c),
            _ => None,
        });
        assert_eq!(
            glyph,
            Some(t.scheme.on_primary),
            "the icon in the label's colour"
        );
        // 16 before the icon, as the reference's `.icon` padding says.
        let icon_x = prims
            .iter()
            .find_map(|p| match p {
                Primitive::Path { path, .. } => path.verbs().iter().find_map(|v| match v {
                    frus_core::PathVerb::MoveTo(pt) => Some(pt.x),
                    _ => None,
                }),
                _ => None,
            })
            .unwrap();
        assert!(icon_x >= surface.x + 16.0 - 0.5, "{icon_x} vs {surface:?}");
    }

    /// **A caller's style outranks the theme's, which outranks the default** — and a
    /// caller's plain colour leaves the disabled look alone.
    #[test]
    fn a_style_outranks_the_theme_which_outranks_the_default() {
        let mut t = phone();
        let red = Color::rgb8(200, 0, 0);
        let blue = Color::rgb8(0, 0, 200);
        t.widgets.button.filled_style =
            Some(ButtonStyle::new().background_color(WidgetStateProperty::all(red)));
        let (_, themed) = laid_out(Button::new("OK").on_press(Msg::Pressed), &t);
        assert_eq!(surface_colour(&themed), Some(red));
        let (_, mine) = laid_out(Button::new("OK").color(blue).on_press(Msg::Pressed), &t);
        assert_eq!(surface_colour(&mine), Some(blue));
        let (_, off) = laid_out(Button::new("OK").color(blue), &phone());
        assert_eq!(
            surface_colour(&off),
            Some(crate::disabled::disabled_container(&phone())),
            "a plain colour is the enabled one"
        );
        // A fixed size is kept.
        let sized = Button::new("OK")
            .style(ButtonStyle::new().fixed_size(Size::new(120.0, 50.0)))
            .on_press(Msg::Pressed);
        let (b, _) = laid_out(sized, &phone());
        assert_eq!((b.width, b.height), (120.0, 50.0));
    }

    /// **A button is a stadium whatever its size**, unless told otherwise.
    #[test]
    fn a_button_is_a_stadium() {
        let (_, prims) = laid_out(Button::new("OK").on_press(Msg::Pressed), &phone());
        let radius = prims.iter().find_map(|p| match p {
            Primitive::Rect { radius, blur, .. } if *blur == 0.0 => Some(*radius),
            _ => None,
        });
        assert_eq!(radius, Some(BorderRadius::uniform(20.0)));
    }
}
