//! [`Button`]: the reference's five buttons, in one widget with a variant.
//!
//! They are one box with one label and differ in **emphasis**: filled for the action a
//! screen is about, tonal for the one beside it, elevated where the surface underneath is
//! busy, outlined for a secondary action, and text for the least of all. Everything else —
//! the 40 px height, the 64 px minimum width, the stadium shape, the `label_large` label,
//! the 24 px of room either side — is shared, which is why they are a variant here and not
//! five types.
//!
//! ```ignore
//! Button::new("Save").on_press(Msg::Save)                              // filled
//! Button::new("Cancel").variant(Variant::Text).on_press(Msg::Cancel)  // text
//! Button::new("Delete").variant(Variant::Danger).on_press(Msg::Delete) // the error role
//! ```
//!
//! Every measurement and colour is overridable, per call or through
//! [`ButtonTheme`](crate::ButtonTheme).

use frus_core::{BorderRadius, Color, Point, Rect, Scene, TextStyle};
use frus_layout::{Dimension, Style};

use crate::disabled::{disabled_container, disabled_content};
use crate::interaction::Status;
use crate::theme::Theme;
use crate::widget::Widget;

/// A button's height, and the smallest it will be.
pub const BUTTON_HEIGHT: f32 = 40.0;
/// The narrowest a button gets, however short its label.
pub const BUTTON_MIN_WIDTH: f32 = 64.0;
/// The room either side of the label.
pub const BUTTON_PADDING: f32 = 24.0;
/// The room either side of a **text** button's label, which has no box to fill.
pub const BUTTON_TEXT_PADDING: f32 = 12.0;
/// How far an elevated button sits off the surface.
pub const BUTTON_ELEVATION: f32 = 1.0;
/// An outlined button's outline.
pub const BUTTON_BORDER_WIDTH: f32 = 1.0;

/// How much of a screen's attention a button is asking for.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Default)]
pub enum Variant {
    /// The accent, filled: the one action a screen is about.
    #[default]
    Filled,
    /// A tonal fill — beside a filled button, not competing with it.
    Tonal,
    /// A raised surface with a shadow, for a button over busy content.
    Elevated,
    /// An outline and no fill.
    Outlined,
    /// A label alone.
    Text,
    /// Filled in the **error** role — a destructive action.
    ///
    /// Not one of the reference's five: there, a destructive button is a filled button
    /// given the error colours by hand. It is here because saying *this action destroys
    /// something* is worth a name, and because the alternative is every application
    /// writing the same two colour overrides.
    Danger,
}

impl Variant {
    /// Whether the variant carries a shadow by default.
    const fn elevation(self) -> f32 {
        match self {
            Variant::Elevated => BUTTON_ELEVATION,
            _ => 0.0,
        }
    }

    /// The room either side of the label.
    const fn padding(self) -> f32 {
        match self {
            Variant::Text => BUTTON_TEXT_PADDING,
            _ => BUTTON_PADDING,
        }
    }
}

/// A clickable button.
pub struct Button<Msg> {
    label: String,
    variant: Variant,
    on_press: Option<Msg>,
    /// Enabled? Disabled (`false`): greyed out, no shadow, no click, no focus.
    enabled: bool,
    label_style: Option<TextStyle>,
    color: Option<Color>,
    label_color: Option<Color>,
    border_color: Option<Color>,
    border_width: Option<f32>,
    radius: Option<BorderRadius>,
    padding: Option<f32>,
    height: Option<f32>,
    min_width: Option<f32>,
    elevation: Option<f32>,
}

impl<Msg> Button<Msg> {
    /// Creates a button with a label.
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            variant: Variant::default(),
            on_press: None,
            enabled: true,
            label_style: None,
            color: None,
            label_color: None,
            border_color: None,
            border_width: None,
            radius: None,
            padding: None,
            height: None,
            min_width: None,
            elevation: None,
        }
    }

    /// Enables or **disables** the button: disabled, it is greyed out, has no shadow
    /// and emits nothing at all (neither click nor keyboard focus) — the rendering of
    /// an unavailable control, e.g. "Next" while a step is invalid.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Overrides the corner radii (uniform via `f32`, per corner via [`BorderRadius`] —
    /// connected segments, button groups…). Defaults to a **stadium**: the radius is half
    /// the button's height, whatever that height turns out to be.
    pub fn radius(mut self, radius: impl Into<BorderRadius>) -> Self {
        self.radius = Some(radius.into());
        self
    }

    /// Chooses the visual variant.
    pub fn variant(mut self, variant: Variant) -> Self {
        self.variant = variant;
        self
    }

    /// The label's type. Defaults to the theme's `label_large` step.
    pub fn label_style(mut self, style: TextStyle) -> Self {
        self.label_style = Some(style);
        self
    }

    /// Font size — sugar for [`Button::label_style`] with the label step resized.
    pub fn size(mut self, size: f32) -> Self {
        let mut style = self.label_style.unwrap_or(TextStyle::new(size));
        style.size = size;
        self.label_style = Some(style);
        self
    }

    /// The surface under the label.
    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }

    /// The label's colour.
    pub fn label_color(mut self, color: Color) -> Self {
        self.label_color = Some(color);
        self
    }

    /// The outline's colour.
    pub fn border_color(mut self, color: Color) -> Self {
        self.border_color = Some(color);
        self
    }

    /// The outline's thickness; `0.0` removes it.
    pub fn border_width(mut self, width: f32) -> Self {
        self.border_width = Some(width);
        self
    }

    /// The room either side of the label.
    pub fn padding(mut self, padding: f32) -> Self {
        self.padding = Some(padding);
        self
    }

    /// The button's height.
    pub fn height(mut self, height: f32) -> Self {
        self.height = Some(height);
        self
    }

    /// The narrowest it will be, however short its label.
    pub fn min_width(mut self, width: f32) -> Self {
        self.min_width = Some(width);
        self
    }

    /// How far it sits off the surface. `0.0` is flat, and flat is what four of the five
    /// variants are.
    pub fn elevation(mut self, elevation: f32) -> Self {
        self.elevation = Some(elevation);
        self
    }

    /// Message emitted on click.
    pub fn on_press(mut self, message: Msg) -> Self {
        self.on_press = Some(message);
        self
    }

    fn label_style_of(&self, theme: &Theme) -> TextStyle {
        self.label_style
            .or(theme.widgets.button.label_style)
            .unwrap_or(theme.text.label_large)
    }

    fn padding_of(&self, theme: &Theme) -> f32 {
        self.padding
            .or(theme.widgets.button.padding)
            .unwrap_or(self.variant.padding())
    }

    fn height_of(&self, theme: &Theme) -> f32 {
        self.height
            .or(theme.widgets.button.height)
            .unwrap_or(BUTTON_HEIGHT)
    }

    fn min_width_of(&self, theme: &Theme) -> f32 {
        self.min_width
            .or(theme.widgets.button.min_width)
            .unwrap_or(BUTTON_MIN_WIDTH)
    }

    fn elevation_of(&self, theme: &Theme) -> f32 {
        if !self.enabled {
            return 0.0;
        }
        self.elevation
            .or(theme.widgets.button.elevation)
            .unwrap_or(self.variant.elevation())
    }

    fn radius_of(&self, theme: &Theme, height: f32) -> BorderRadius {
        self.radius
            .or(theme.widgets.button.radius)
            // A stadium, which is what the reference's buttons are: the radius follows the
            // height, so a shorter button stays a lozenge rather than becoming a box with
            // soft corners.
            .unwrap_or(BorderRadius::uniform(height / 2.0))
    }

    /// `(background, label, outline)` for the variant and the theme, disabled included.
    ///
    /// A disabled control is the same in every variant: the reference flattens all five
    /// to `on_surface` at 12 % under a label at 38 %, so that unavailable reads as
    /// unavailable rather than as a quieter version of the variant.
    fn palette(&self, theme: &Theme) -> (Color, Color, Option<Color>) {
        if !self.enabled {
            return (
                disabled_container(theme),
                disabled_content(theme),
                None,
            );
        }
        let (background, label, outline) = match self.variant {
            Variant::Filled => (theme.scheme.primary, theme.scheme.on_primary, None),
            Variant::Tonal => (
                theme.scheme.secondary_container,
                theme.scheme.on_secondary_container,
                None,
            ),
            Variant::Elevated => (theme.scheme.surface_container, theme.scheme.primary, None),
            Variant::Outlined => (
                Color::TRANSPARENT,
                theme.scheme.primary,
                Some(theme.scheme.outline),
            ),
            Variant::Text => (Color::TRANSPARENT, theme.scheme.primary, None),
            Variant::Danger => (theme.scheme.error, theme.scheme.on_error, None),
        };
        (
            self.color
                .or(theme.widgets.button.color)
                .unwrap_or(background),
            self.label_color
                .or(theme.widgets.button.label_color)
                .unwrap_or(label),
            self.border_color
                .or(theme.widgets.button.border_color)
                .or(outline),
        )
    }
}

impl<Msg: Clone> Widget<Msg> for Button<Msg> {
    fn style(&self) -> Style {
        Widget::<Msg>::style_themed(self, &Theme::default())
    }

    fn style_themed(&self, theme: &Theme) -> Style {
        let style = self.label_style_of(theme);
        let measured =
            frus_text::measure_styled(&self.label, style.size, style.weight, style.italic);
        let width = (measured.width + self.padding_of(theme) * 2.0).max(self.min_width_of(theme));
        Style {
            width: Dimension::Length(width.ceil()),
            height: Dimension::Length(self.height_of(theme).max(measured.height).ceil()),
            ..Default::default()
        }
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &[]
    }

    fn paint(&self, bounds: Rect, status: Status, theme: &Theme, scene: &mut Scene) {
        let o = status.opacity;
        let (base, on_color, border) = self.palette(theme);
        let radius = self.radius_of(theme, bounds.height);
        let elevation = self.elevation_of(theme);

        // The shadow belongs to **one** variant. Every enabled button used to cast one,
        // which is the reference's elevated button drawn five times over.
        if elevation > 0.0 {
            let blur = elevation * 4.0 + 8.0;
            scene.shadow(
                Rect::new(
                    bounds.x - blur,
                    bounds.y + elevation * 2.0 - blur,
                    bounds.width + 2.0 * blur,
                    bounds.height + 2.0 * blur,
                ),
                theme.scheme.shadow.with_alpha(0.35).fade(o),
                radius.inflate(blur),
                blur,
            );
        }

        // Hover/press/focus through the theme's baked state layer. A disabled control has
        // no states to layer, and a transparent surface must stay transparent at rest:
        // the layer is what tints it under the pointer.
        let color = if self.enabled {
            theme.state_layer(base, on_color, &status)
        } else {
            base
        };
        let (border_width, border_color) = match (
            self.border_width.or(theme.widgets.button.border_width),
            border,
        ) {
            (Some(width), Some(color)) => (width, color),
            (Some(width), None) => (width, theme.scheme.outline),
            (None, Some(color)) => (BUTTON_BORDER_WIDTH, color),
            (None, None) => (0.0, Color::TRANSPARENT),
        };
        scene.draw_rect(
            bounds,
            color.fade(o),
            radius,
            border_width,
            border_color.fade(o),
        );

        // Centred, both ways: a label pinned to the padding drifts off centre the moment
        // the button is given a width of its own.
        let style = self.label_style_of(theme);
        let measured =
            frus_text::measure_styled(&self.label, style.size, style.weight, style.italic);
        scene.text_styled(
            Point::new(
                bounds.x + (bounds.width - measured.width) / 2.0,
                bounds.y + (bounds.height - measured.height) / 2.0,
            ),
            self.label.clone(),
            &style,
            on_color.fade(o),
        );
    }

    fn on_click(&self) -> Option<Msg> {
        if self.enabled {
            self.on_press.clone()
        } else {
            None
        }
    }

    fn ink(&self, theme: &Theme) -> Option<crate::InkStyle> {
        // A disabled control does not answer a tap, so it does not splash either.
        if !self.enabled {
            return None;
        }
        // The splash takes the button's **own** label colour: white-ish ink on a filled
        // button, the accent on a text one. The theme's default (`on_surface`) would
        // vanish on the first and shout on the second.
        let (_, on_color, _) = self.palette(theme);
        // Unless the application has named one: a theme that says what ink looks like has
        // said it for every surface, not for the plain ones only.
        let splash = theme
            .widgets
            .ink
            .color
            .unwrap_or_else(|| on_color.fade(0.16));
        Some(
            crate::InkStyle::of(theme)
                .color(splash)
                .radius(self.radius_of(theme, self.height_of(theme))),
        )
    }

    fn focusable(&self) -> bool {
        self.enabled
    }

    fn semantics(&self) -> Option<frus_core::Semantics> {
        let semantics =
            frus_core::Semantics::new(frus_core::Role::Button).label(self.label.clone());
        // A disabled button does not announce a clickable action.
        Some(if self.enabled {
            semantics.clickable()
        } else {
            semantics.disabled(true)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use frus_core::Primitive;

    #[derive(Clone, Debug, PartialEq)]
    enum Msg {
        Pressed,
    }

    /// What a button paints, in order: the shadow if it has one, then its box, then its
    /// label.
    fn painted(button: &Button<Msg>) -> Vec<Primitive> {
        let mut scene = Scene::new();
        Widget::<Msg>::paint(
            button,
            Rect::new(0.0, 0.0, 120.0, BUTTON_HEIGHT),
            Status::default(),
            &Theme::default(),
            &mut scene,
        );
        scene.primitives().to_vec()
    }

    fn surface(button: &Button<Msg>) -> (Color, BorderRadius, f32) {
        painted(button)
            .iter()
            .find_map(|p| match p {
                Primitive::Rect {
                    color,
                    radius,
                    border_width,
                    blur,
                    ..
                } if *blur == 0.0 => Some((*color, *radius, *border_width)),
                _ => None,
            })
            .expect("a button paints its box")
    }

    #[test]
    fn on_click_returns_message() {
        let button = Button::new("OK").on_press(Msg::Pressed);
        assert_eq!(Widget::on_click(&button), Some(Msg::Pressed));
    }

    #[test]
    fn disabled_button_is_inert_and_unfocusable() {
        let button = Button::new("Next").on_press(Msg::Pressed).enabled(false);
        assert_eq!(Widget::on_click(&button), None, "disabled: no message");
        assert!(
            !Widget::<Msg>::focusable(&button),
            "disabled: out of the tab order"
        );
        let semantics = Widget::<Msg>::semantics(&button).expect("semantics present");
        assert!(!semantics.clickable, "disabled: not announced as clickable");
        assert!(semantics.disabled, "and announced as unavailable");
        let enabled = Button::new("Next").on_press(Msg::Pressed).enabled(true);
        assert_eq!(Widget::on_click(&enabled), Some(Msg::Pressed));
    }

    #[test]
    fn only_the_elevated_variant_casts_a_shadow() {
        // Every enabled button used to cast one, which is the reference's *elevated*
        // button drawn five times over — and an emphasis order in which nothing is quiet.
        let shadows = |variant: Variant| {
            painted(&Button::<Msg>::new("Go").variant(variant))
                .iter()
                .filter(|p| matches!(p, Primitive::Rect { blur, .. } if *blur > 0.0))
                .count()
        };
        assert_eq!(shadows(Variant::Elevated), 1);
        for flat in [
            Variant::Filled,
            Variant::Tonal,
            Variant::Outlined,
            Variant::Text,
            Variant::Danger,
        ] {
            assert_eq!(shadows(flat), 0, "{flat:?} is flat");
        }
        assert_eq!(
            shadows(Variant::Elevated).min(
                painted(
                    &Button::<Msg>::new("Go")
                        .variant(Variant::Elevated)
                        .enabled(false)
                )
                .iter()
                .filter(|p| matches!(p, Primitive::Rect { blur, .. } if *blur > 0.0))
                .count()
            ),
            0,
            "and a disabled one does not float"
        );
    }

    #[test]
    fn the_variants_are_told_apart_by_their_surface() {
        let theme = Theme::default();
        assert_eq!(surface(&Button::<Msg>::new("Go")).0, theme.scheme.primary);
        assert_eq!(
            surface(&Button::<Msg>::new("Go").variant(Variant::Tonal)).0,
            theme.scheme.secondary_container
        );
        assert_eq!(
            surface(&Button::<Msg>::new("Go").variant(Variant::Danger)).0,
            theme.scheme.error
        );
        // The two quiet ones have no surface at all until something happens to them.
        assert_eq!(
            surface(&Button::<Msg>::new("Go").variant(Variant::Text)).0,
            Color::TRANSPARENT
        );
        let outlined = surface(&Button::<Msg>::new("Go").variant(Variant::Outlined));
        assert_eq!(outlined.0, Color::TRANSPARENT);
        assert_eq!(
            outlined.2, BUTTON_BORDER_WIDTH,
            "an outline, and only there"
        );
        assert_eq!(
            surface(&Button::<Msg>::new("Go").variant(Variant::Text)).2,
            0.0
        );
    }

    #[test]
    fn a_button_is_a_stadium_whatever_its_height() {
        // The radius follows the height rather than the theme's corner setting: half of it,
        // so a button stays a lozenge instead of becoming a box with soft corners.
        assert_eq!(
            surface(&Button::<Msg>::new("Go")).1,
            BorderRadius::uniform(BUTTON_HEIGHT / 2.0)
        );
        // And a caller who wants corners can still have them.
        assert_eq!(
            surface(&Button::<Msg>::new("Go").radius(4.0)).1,
            BorderRadius::uniform(4.0)
        );
    }

    #[test]
    fn a_button_is_the_references_size() {
        let theme = Theme::default();
        let size = |button: Button<Msg>| {
            let style = Widget::<Msg>::style_themed(&button, &theme);
            match (style.width, style.height) {
                (Dimension::Length(w), Dimension::Length(h)) => (w, h),
                other => panic!("{other:?}"),
            }
        };
        let (width, height) = size(Button::new("OK"));
        assert_eq!(height, BUTTON_HEIGHT);
        // The minimum binds only where the label plus its room falls short of it, which
        // at 24 px either side means a label of about one character.
        assert!(width >= BUTTON_MIN_WIDTH);
        assert_eq!(
            size(Button::new("I")).0,
            BUTTON_MIN_WIDTH,
            "a one-letter label does not give a narrower button"
        );
        // A long one grows past the minimum, by the padding on either side.
        let (wide, _) = size(Button::new("A considerably longer label"));
        assert!(wide > BUTTON_MIN_WIDTH);
        // A text button keeps the height and gives back the room.
        let (narrow, _) = size(Button::new("A considerably longer label").variant(Variant::Text));
        assert_eq!(
            wide - narrow,
            (BUTTON_PADDING - BUTTON_TEXT_PADDING) * 2.0,
            "a text button's label sits closer to its edges"
        );
    }

    #[test]
    fn a_disabled_button_looks_the_same_in_every_variant() {
        // Unavailable has to read as unavailable, not as a quieter version of the variant.
        let theme = Theme::default();
        let grey = theme.scheme.on_surface.fade(0.12);
        for variant in [
            Variant::Filled,
            Variant::Tonal,
            Variant::Elevated,
            Variant::Outlined,
            Variant::Text,
            Variant::Danger,
        ] {
            assert_eq!(
                surface(&Button::<Msg>::new("Go").variant(variant).enabled(false)).0,
                grey,
                "{variant:?}"
            );
        }
    }

    #[test]
    fn every_measurement_is_the_callers_and_then_the_themes() {
        let mut theme = Theme::default();
        theme.widgets.button.height = Some(56.0);
        let height =
            |button: Button<Msg>, theme: &Theme| match Widget::<Msg>::style_themed(&button, theme)
                .height
            {
                Dimension::Length(h) => h,
                other => panic!("{other:?}"),
            };
        assert_eq!(
            height(Button::new("Go"), &Theme::default()),
            BUTTON_HEIGHT,
            "the framework's"
        );
        assert_eq!(height(Button::new("Go"), &theme), 56.0, "the theme's");
        assert_eq!(
            height(Button::new("Go").height(30.0), &theme),
            30.0,
            "the caller's, over the theme's"
        );
    }
}
