//! [`IconButton`]: a button holding **one glyph**, in a box as wide as it is tall.
//!
//! It exists because milestone 313 kept finding the same thing. A `Button` is sized for a
//! word — 64 px wide before its label is measured, 24 px of room either side — and a back
//! arrow, a month arrow, a delete cross or a stepper's plus is not a word. Seven call sites
//! in this repository were asking a button to be something it is not, and the reference has
//! had the answer all along: 40 × 40, circular, no fill, the icon in `on_surface_variant`.
//!
//! ```ignore
//! IconButton::new(Icons::Close).label("Remove").on_press(Msg::Delete)
//! IconButton::glyph("\u{2190}").label("Back").on_press(Msg::Pop)
//! IconButton::new(Icons::Star).selected(starred).on_press(Msg::Star)
//! ```
//!
//! It takes either an icon from the bundled set or a **glyph**, because the set does not
//! cover everything an application draws and a button that only accepts what happens to be
//! bundled is a button that sends people back to `Button`.
//!
//! **A label is not optional in practice.** An icon says nothing to a screen reader, so
//! [`IconButton::label`] is what makes it announceable — the reference's `tooltip`, doing
//! the same job.

use frus_core::{BorderRadius, Color, Point, Rect, Scene, TextStyle};
use frus_layout::{Dimension, Style};

use crate::disabled::{disabled_container, disabled_content};
use crate::icons::Icons;
use crate::interaction::Status;
use crate::theme::Theme;
use crate::widget::Widget;

/// The box: as wide as it is tall.
pub const ICON_BUTTON_SIZE: f32 = 40.0;
/// The glyph inside it.
pub const ICON_BUTTON_ICON_SIZE: f32 = 24.0;
/// An outlined icon button's outline.
pub const ICON_BUTTON_BORDER_WIDTH: f32 = 1.0;

/// The icon grid the vector icons are drawn on; see [`crate::icons`].
const ICON_GRID: f32 = 24.0;

/// How much of a surface an icon button brings with it.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Default)]
pub enum IconButtonVariant {
    /// No surface at all: the glyph, and the ink a tap leaves.
    #[default]
    Standard,
    /// The accent, filled.
    Filled,
    /// A tonal fill.
    Tonal,
    /// An outline and no fill.
    Outlined,
}

/// What the button draws: one of the bundled icons, or a glyph of your own.
enum Content {
    Icon(Icons),
    Glyph(String),
}

/// A button holding one glyph.
pub struct IconButton<Msg> {
    content: Content,
    variant: IconButtonVariant,
    selected: bool,
    enabled: bool,
    label: Option<String>,
    on_press: Option<Msg>,
    size: Option<f32>,
    icon_size: Option<f32>,
    color: Option<Color>,
    icon_color: Option<Color>,
    border_color: Option<Color>,
    border_width: Option<f32>,
    radius: Option<BorderRadius>,
}

impl<Msg> IconButton<Msg> {
    /// A button showing one of the bundled icons.
    pub fn new(icon: Icons) -> Self {
        Self::of(Content::Icon(icon))
    }

    /// A button showing a **glyph** — an arrow, a cross, an ellipsis — for the many shapes
    /// the bundled set does not carry.
    pub fn glyph(glyph: impl Into<String>) -> Self {
        Self::of(Content::Glyph(glyph.into()))
    }

    fn of(content: Content) -> Self {
        Self {
            content,
            variant: IconButtonVariant::default(),
            selected: false,
            enabled: true,
            label: None,
            on_press: None,
            size: None,
            icon_size: None,
            color: None,
            icon_color: None,
            border_color: None,
            border_width: None,
            radius: None,
        }
    }

    /// What a screen reader announces. An icon has no text of its own, so without this the
    /// button is a shape with no name.
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Chooses how much surface the button brings.
    pub fn variant(mut self, variant: IconButtonVariant) -> Self {
        self.variant = variant;
        self
    }

    /// Marks it **on** — a toggle that is currently pressed in. The glyph takes the accent.
    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    /// Enables or disables it. Disabled, it is greyed out and emits nothing.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// The message emitted on click.
    pub fn on_press(mut self, message: Msg) -> Self {
        self.on_press = Some(message);
        self
    }

    /// The box's side.
    pub fn size(mut self, size: f32) -> Self {
        self.size = Some(size);
        self
    }

    /// The glyph's size inside it.
    pub fn icon_size(mut self, size: f32) -> Self {
        self.icon_size = Some(size);
        self
    }

    /// The surface under the glyph.
    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }

    /// The glyph's colour.
    pub fn icon_color(mut self, color: Color) -> Self {
        self.icon_color = Some(color);
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

    /// The corner radii. Unset, the button is a **circle**.
    pub fn radius(mut self, radius: impl Into<BorderRadius>) -> Self {
        self.radius = Some(radius.into());
        self
    }

    fn side(&self, theme: &Theme) -> f32 {
        self.size
            .or(theme.widgets.icon_button.size)
            .unwrap_or(ICON_BUTTON_SIZE)
    }

    fn glyph_size(&self, theme: &Theme) -> f32 {
        self.icon_size
            .or(theme.widgets.icon_button.icon_size)
            .unwrap_or(ICON_BUTTON_ICON_SIZE)
    }

    fn radius_of(&self, theme: &Theme, side: f32) -> BorderRadius {
        self.radius
            .or(theme.widgets.icon_button.radius)
            // A circle, which is what an icon button is: the radius follows the box, so a
            // smaller one stays round instead of becoming a square with soft corners.
            .unwrap_or(BorderRadius::uniform(side / 2.0))
    }

    /// `(surface, glyph, outline)`.
    fn palette(&self, theme: &Theme) -> (Color, Color, Option<Color>) {
        if !self.enabled {
            return (
                match self.variant {
                    IconButtonVariant::Standard | IconButtonVariant::Outlined => Color::TRANSPARENT,
                    _ => disabled_container(theme),
                },
                disabled_content(theme),
                // An outlined icon button **keeps its outline**, at the container opacity.
                // Dropping it took the shape away entirely: milestone 324's golden showed a
                // disabled stepper as two bare glyphs with no buttons around them, which
                // reads as broken rather than as unavailable.
                match self.variant {
                    IconButtonVariant::Outlined => Some(disabled_container(theme)),
                    _ => None,
                },
            );
        }
        let (surface, glyph, outline) = match self.variant {
            IconButtonVariant::Standard => (
                Color::TRANSPARENT,
                if self.selected {
                    theme.scheme.primary
                } else {
                    theme.scheme.on_surface_variant
                },
                None,
            ),
            IconButtonVariant::Filled => (theme.scheme.primary, theme.scheme.on_primary, None),
            IconButtonVariant::Tonal => (
                theme.scheme.secondary_container,
                theme.scheme.on_secondary_container,
                None,
            ),
            IconButtonVariant::Outlined => (
                Color::TRANSPARENT,
                if self.selected {
                    theme.scheme.primary
                } else {
                    theme.scheme.on_surface_variant
                },
                Some(theme.scheme.outline),
            ),
        };
        (
            self.color
                .or(theme.widgets.icon_button.color)
                .unwrap_or(surface),
            self.icon_color
                .or(theme.widgets.icon_button.icon_color)
                .unwrap_or(glyph),
            self.border_color
                .or(theme.widgets.icon_button.border_color)
                .or(outline),
        )
    }
}

impl<Msg: Clone> Widget<Msg> for IconButton<Msg> {
    fn style(&self) -> Style {
        Widget::<Msg>::style_themed(self, &Theme::default())
    }

    fn style_themed(&self, theme: &Theme) -> Style {
        let side = self.side(theme);
        Style {
            width: Dimension::Length(side),
            height: Dimension::Length(side),
            ..Default::default()
        }
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &[]
    }

    fn paint(&self, bounds: Rect, status: Status, theme: &Theme, scene: &mut Scene) {
        let o = status.opacity;
        let (surface, glyph, outline) = self.palette(theme);
        let radius = self.radius_of(theme, bounds.height.min(bounds.width));
        let (border_width, border_color) = match (
            self.border_width.or(theme.widgets.icon_button.border_width),
            outline,
        ) {
            (Some(width), Some(color)) => (width, color),
            (Some(width), None) => (width, theme.scheme.outline),
            (None, Some(color)) => (ICON_BUTTON_BORDER_WIDTH, color),
            (None, None) => (0.0, Color::TRANSPARENT),
        };
        let surface = if self.enabled {
            theme.state_layer(surface, glyph, &status)
        } else {
            surface
        };
        scene.draw_rect(
            bounds,
            surface.fade(o),
            radius,
            border_width,
            border_color.fade(o),
        );

        let size = self.glyph_size(theme);
        match &self.content {
            Content::Icon(name) => {
                let path = name.path().scaled(size / ICON_GRID).translated(
                    bounds.x + (bounds.width - size) / 2.0,
                    bounds.y + (bounds.height - size) / 2.0,
                );
                scene.fill_path(&path, glyph.fade(o));
            }
            Content::Glyph(text) => {
                let style = TextStyle::new(size);
                let measured = frus_text::measure_styled(text, size, style.weight, style.italic);
                scene.text_styled(
                    Point::new(
                        bounds.x + (bounds.width - measured.width) / 2.0,
                        bounds.y + (bounds.height - measured.height) / 2.0,
                    ),
                    text.clone(),
                    &style,
                    glyph.fade(o),
                );
            }
        }
    }

    fn on_click(&self) -> Option<Msg> {
        if self.enabled {
            self.on_press.clone()
        } else {
            None
        }
    }

    fn ink(&self, theme: &Theme) -> Option<crate::InkStyle> {
        if !self.enabled {
            return None;
        }
        let (_, glyph, _) = self.palette(theme);
        let splash = theme.widgets.ink.color.unwrap_or_else(|| glyph.fade(0.16));
        Some(
            crate::InkStyle::of(theme)
                .color(splash)
                .radius(self.radius_of(theme, self.side(theme))),
        )
    }

    fn focusable(&self) -> bool {
        self.enabled
    }

    fn semantics(&self) -> Option<frus_core::SemanticsProperties> {
        let semantics = frus_core::SemanticsProperties::new(frus_core::Role::Button);
        let semantics = match &self.label {
            Some(label) => semantics.label(label.clone()),
            // Better than nothing, and visibly worse than a label: a glyph read out is at
            // least something, an unnamed icon is silence.
            None => match &self.content {
                Content::Glyph(text) => semantics.label(text.clone()),
                Content::Icon(_) => semantics,
            },
        };
        Some(if self.enabled {
            semantics.toggled(self.selected).clickable()
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

    fn painted(button: &IconButton<Msg>) -> Vec<Primitive> {
        let mut scene = Scene::new();
        Widget::<Msg>::paint(
            button,
            Rect::new(0.0, 0.0, ICON_BUTTON_SIZE, ICON_BUTTON_SIZE),
            Status::default(),
            &Theme::default(),
            &mut scene,
        );
        scene.primitives().to_vec()
    }

    #[test]
    fn it_is_a_circle_as_wide_as_it_is_tall() {
        let theme = Theme::default();
        let style = Widget::<Msg>::style_themed(&IconButton::new(Icons::Close), &theme);
        assert_eq!(style.width, Dimension::Length(ICON_BUTTON_SIZE));
        assert_eq!(style.height, Dimension::Length(ICON_BUTTON_SIZE));
        let radius = painted(&IconButton::new(Icons::Close))
            .iter()
            .find_map(|p| match p {
                Primitive::Rect { radius, .. } => Some(*radius),
                _ => None,
            })
            .expect("a box");
        assert_eq!(radius, BorderRadius::uniform(ICON_BUTTON_SIZE / 2.0));
    }

    #[test]
    fn a_standard_one_is_a_glyph_and_nothing_else() {
        let theme = Theme::default();
        let painted = painted(&IconButton::new(Icons::Close));
        let (color, border) = painted
            .iter()
            .find_map(|p| match p {
                Primitive::Rect {
                    color,
                    border_width,
                    ..
                } => Some((*color, *border_width)),
                _ => None,
            })
            .expect("a box");
        assert_eq!(color, Color::TRANSPARENT, "no surface at rest");
        assert_eq!(border, 0.0);
        let glyph = painted
            .iter()
            .find_map(|p| match p {
                Primitive::Path { fill, .. } => *fill,
                _ => None,
            })
            .expect("an icon");
        assert_eq!(glyph, theme.scheme.on_surface_variant);
    }

    #[test]
    fn a_glyph_is_drawn_where_the_set_has_no_icon() {
        // The bundled set has no back arrow, no minus, no ellipsis. A button that only took
        // what happens to be bundled would send those call sites back to `Button`.
        let painted = painted(&IconButton::<Msg>::glyph("\u{2190}"));
        assert!(
            painted
                .iter()
                .any(|p| matches!(p, Primitive::Text { text, .. } if text == "\u{2190}")),
            "the glyph is drawn"
        );
    }

    #[test]
    fn selected_takes_the_accent() {
        let theme = Theme::default();
        let glyph = |selected: bool| {
            painted(&IconButton::<Msg>::new(Icons::Star).selected(selected))
                .iter()
                .find_map(|p| match p {
                    Primitive::Path { fill, .. } => *fill,
                    _ => None,
                })
                .expect("an icon")
        };
        assert_eq!(glyph(false), theme.scheme.on_surface_variant);
        assert_eq!(glyph(true), theme.scheme.primary);
    }

    #[test]
    fn the_variants_are_told_apart_by_their_surface() {
        let theme = Theme::default();
        let surface = |variant: IconButtonVariant| {
            painted(&IconButton::<Msg>::new(Icons::Close).variant(variant))
                .iter()
                .find_map(|p| match p {
                    Primitive::Rect {
                        color,
                        border_width,
                        ..
                    } => Some((*color, *border_width)),
                    _ => None,
                })
                .expect("a box")
        };
        assert_eq!(surface(IconButtonVariant::Filled).0, theme.scheme.primary);
        assert_eq!(
            surface(IconButtonVariant::Tonal).0,
            theme.scheme.secondary_container
        );
        let outlined = surface(IconButtonVariant::Outlined);
        assert_eq!(outlined.0, Color::TRANSPARENT);
        assert_eq!(outlined.1, ICON_BUTTON_BORDER_WIDTH);
    }

    #[test]
    fn disabled_is_inert_and_says_so() {
        let button = IconButton::new(Icons::Close)
            .on_press(Msg::Pressed)
            .enabled(false);
        assert_eq!(Widget::on_click(&button), None);
        assert!(!Widget::<Msg>::focusable(&button));
        assert!(Widget::<Msg>::ink(&button, &Theme::default()).is_none());
        let semantics = Widget::<Msg>::semantics(&button).expect("semantics");
        assert!(semantics.disabled);
        assert!(!semantics.clickable);
    }

    #[test]
    fn an_icon_button_can_be_named() {
        // An icon says nothing to a screen reader; the label is what it is announced by.
        let named = IconButton::<Msg>::new(Icons::Close).label("Remove");
        assert_eq!(
            Widget::<Msg>::semantics(&named).unwrap().label.as_deref(),
            Some("Remove")
        );
        // A glyph at least reads as itself.
        assert_eq!(
            Widget::<Msg>::semantics(&IconButton::<Msg>::glyph("+"))
                .unwrap()
                .label
                .as_deref(),
            Some("+")
        );
    }

    #[test]
    fn every_measurement_is_the_callers_and_then_the_themes() {
        let mut theme = Theme::default();
        theme.widgets.icon_button.size = Some(48.0);
        let side = |button: IconButton<Msg>, theme: &Theme| match Widget::<Msg>::style_themed(
            &button, theme,
        )
        .width
        {
            Dimension::Length(w) => w,
            other => panic!("{other:?}"),
        };
        assert_eq!(
            side(IconButton::new(Icons::Close), &Theme::default()),
            ICON_BUTTON_SIZE,
            "the framework's"
        );
        assert_eq!(
            side(IconButton::new(Icons::Close), &theme),
            48.0,
            "the theme's"
        );
        assert_eq!(
            side(IconButton::new(Icons::Close).size(32.0), &theme),
            32.0,
            "the caller's, over the theme's"
        );
    }
}
