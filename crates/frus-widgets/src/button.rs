//! [`Button`]: a themed button with a label, variants and interaction states.

use frus_core::{BorderRadius, Color, Point, Rect, Scene};
use frus_layout::{Dimension, Style};

use crate::interaction::Status;
use crate::theme::Theme;
use crate::widget::Widget;

const PAD_X: f32 = 20.0;
const PAD_Y: f32 = 12.0;

/// The visual variant of a button.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Default)]
pub enum Variant {
    /// The theme's main accent.
    #[default]
    Primary,
    /// A neutral surface with a border.
    Secondary,
    /// Action destructive.
    Danger,
}

/// A clickable button.
pub struct Button<Msg> {
    label: String,
    size: f32,
    variant: Variant,
    /// Overridden radii; `None` = the theme's radius (uniform).
    radius: Option<BorderRadius>,
    on_press: Option<Msg>,
    /// Enabled? Disabled (`false`): greyed out, no shadow, no click, no focus.
    enabled: bool,
}

impl<Msg> Button<Msg> {
    /// Creates a button with a label.
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            size: 18.0,
            variant: Variant::Primary,
            radius: None,
            on_press: None,
            enabled: true,
        }
    }

    /// Enables or **disables** the button: disabled, it is greyed out, has no shadow
    /// and emits nothing at all (neither click nor keyboard focus) — the rendering of
    /// an unavailable control (Material style), e.g. "Next" while a step is invalid.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Overrides the corner radii (uniform via `f32`, per corner via
    /// [`BorderRadius`] — connected segments, button groups…). Defaults to
    /// the theme's radius.
    pub fn radius(mut self, radius: impl Into<BorderRadius>) -> Self {
        self.radius = Some(radius.into());
        self
    }

    /// Choisit la variante visuelle.
    pub fn variant(mut self, variant: Variant) -> Self {
        self.variant = variant;
        self
    }

    /// Font size.
    pub fn size(mut self, size: f32) -> Self {
        self.size = size;
        self
    }

    /// Message emitted on click.
    pub fn on_press(mut self, message: Msg) -> Self {
        self.on_press = Some(message);
        self
    }

    /// (base, text, border) according to the variant and the theme.
    fn palette(&self, theme: &Theme) -> (Color, Color, Option<Color>) {
        match self.variant {
            Variant::Primary => (theme.primary, theme.on_primary, None),
            Variant::Secondary => (theme.surface, theme.on_surface, Some(theme.border)),
            Variant::Danger => (theme.error, theme.on_error, None),
        }
    }
}

impl<Msg: Clone> Widget<Msg> for Button<Msg> {
    fn style(&self) -> Style {
        let measured = frus_text::measure(&self.label, self.size);
        Style {
            width: Dimension::Length((measured.width + PAD_X * 2.0).ceil()),
            height: Dimension::Length((measured.height + PAD_Y * 2.0).ceil()),
            ..Default::default()
        }
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &[]
    }

    fn paint(&self, bounds: Rect, status: Status, theme: &Theme, scene: &mut Scene) {
        let o = status.opacity;
        let (base, on_color, border) = self.palette(theme);
        let radius = self.radius.unwrap_or_else(|| theme.radius.into());

        // Disabled: flat neutral fill, discreet text, **no shadow** — an unavailable control.
        if !self.enabled {
            let fill = theme.surface.lerp(theme.muted, 0.12);
            scene.draw_rect(bounds, fill.fade(o), radius, 1.0, theme.border.fade(o));
            scene.text(
                Point::new(bounds.x + PAD_X, bounds.y + PAD_Y),
                self.label.clone(),
                self.size,
                theme.muted.fade(o),
            );
            return;
        }

        // Hover/press/focus state through the theme's baked state layer.
        let color = theme.state_layer(base, on_color, &status);

        let blur = 10.0;
        let shadow_rect = Rect::new(
            bounds.x - blur,
            bounds.y + 3.0 - blur,
            bounds.width + 2.0 * blur,
            bounds.height + 2.0 * blur,
        );
        scene.shadow(
            shadow_rect,
            theme.scheme.shadow.with_alpha(0.35).fade(o),
            radius.inflate(blur),
            blur,
        );
        let (bw, bc) = match border {
            Some(c) => (1.0, c.fade(o)),
            None => (0.0, Color::TRANSPARENT),
        };
        scene.draw_rect(bounds, color.fade(o), radius, bw, bc);
        scene.text(
            Point::new(bounds.x + PAD_X, bounds.y + PAD_Y),
            self.label.clone(),
            self.size,
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
        // The splash takes the button's **own** `on` colour: white-ish ink on a filled
        // primary button, dark ink on a pale secondary one. The theme's default
        // (`on_surface`) would vanish on the first and shout on the third.
        let (_, on_color, _) = self.palette(theme);
        Some(
            crate::InkStyle::of(theme)
                .color(on_color.fade(0.16))
                .radius(self.radius.unwrap_or_else(|| theme.radius.into())),
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
            semantics
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Debug, PartialEq)]
    enum Msg {
        Pressed,
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
        // Semantics with no clickable action.
        let semantics = Widget::<Msg>::semantics(&button).expect("semantics present");
        assert!(!semantics.clickable, "disabled: not announced as clickable");
        // Re-enabled: the click comes back.
        let enabled = Button::new("Next").on_press(Msg::Pressed).enabled(true);
        assert_eq!(Widget::on_click(&enabled), Some(Msg::Pressed));
    }

    #[test]
    fn disabled_button_paints_no_shadow() {
        use frus_core::Primitive;
        let paint = |enabled: bool| {
            let button = Button::<Msg>::new("Next").enabled(enabled);
            let mut scene = Scene::new();
            Widget::<Msg>::paint(
                &button,
                Rect::new(0.0, 0.0, 90.0, 44.0),
                Status::default(),
                &Theme::default(),
                &mut scene,
            );
            scene
                .primitives()
                .iter()
                .any(|p| matches!(p, Primitive::Rect { blur, .. } if *blur > 0.0))
        };
        assert!(paint(true), "enabled: a shadow is drawn");
        assert!(!paint(false), "disabled: no shadow");
    }
}
