//! [`Icon`]: displays a vector icon from the bundled set ([`Icons`]), scaled and
//! coloured to the theme. It is the first consumer of the vector paths
//! ([`frus_core::Path`]) on the widget side.

use frus_core::{Color, Rect, Scene};
use frus_layout::{Dimension, Style};

use crate::icons::{AnimatedIconData, IconData, GRID};
use crate::interaction::Status;
use crate::theme::Theme;
use crate::widget::Widget;

/// A vector icon. Size and colour can both be customised; they default to `24` px
/// and the theme's foreground colour (`on_surface`).
pub struct Icon {
    icon: IconData,
    /// A pair being crossed, and the end it is heading for. `None` for the ordinary
    /// case: an icon that is one mark and stays it.
    morph: Option<(AnimatedIconData, f32)>,
    /// `None` = whatever the theme says, else the 24 px grid the paths are drawn on.
    size: Option<f32>,
    color: Option<Color>,
}

impl Icon {
    /// An icon in the theme's colour and at the theme's size — 24 px unless a theme
    /// says otherwise.
    pub fn new(icon: IconData) -> Self {
        Self {
            icon,
            morph: None,
            size: None,
            color: None,
        }
    }

    /// **An icon that crosses between two marks**, at the end `on` names: `false` is the
    /// pair's `0.0` and `true` its `1.0`.
    ///
    /// The way across is not this widget's to time. It declares the end it wants and the
    /// runtime drives the value there — the same machinery a switch's knob and a drawer's
    /// slide are on, with the same duration and the same curve — so an icon and the panel
    /// it opens move together because they are being driven by the same rule, not because
    /// two timers were set to the same number.
    ///
    /// ```
    /// use frus_widgets::{AnimatedIcons, Icon};
    ///
    /// let open = true;
    /// let _button_mark = Icon::animated(AnimatedIcons::MENU_CLOSE, open);
    /// ```
    pub fn animated(icon: AnimatedIconData, on: bool) -> Self {
        let target = if on { 1.0 } else { 0.0 };
        Self {
            // The end of the animation, which is what an isolated frame — a test, a
            // golden, the first frame after a mount — should draw.
            icon: icon.at(target),
            morph: Some((icon, target)),
            size: None,
            color: None,
        }
    }

    /// Sets the size, that is, the square's side, in logical pixels. Outranks the theme.
    pub fn size(mut self, size: f32) -> Self {
        self.size = Some(size);
        self
    }

    /// Forces the colour; the theme's otherwise, and `on_surface` if it says nothing.
    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }

    /// The box a given side asks for.
    fn sized(&self, side: f32) -> Style {
        Style {
            width: Dimension::Length(side),
            height: Dimension::Length(side),
            ..Default::default()
        }
    }

    /// The side actually drawn: `caller ?? theme ?? the grid`.
    fn resolved_size(&self, theme: Option<&Theme>) -> f32 {
        self.size
            .or_else(|| theme.and_then(|t| t.widgets.icon.size))
            .unwrap_or(GRID)
    }
}

impl<Msg> Widget<Msg> for Icon {
    fn style(&self) -> Style {
        self.sized(self.resolved_size(None))
    }

    /// The theme has a say in the **size**, not only the colour, so an app bar can make
    /// its glyphs smaller and have them take less room rather than the same room with a
    /// smaller drawing in it.
    fn style_themed(&self, theme: &Theme) -> Style {
        self.sized(self.resolved_size(Some(theme)))
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &[]
    }

    fn paint(&self, bounds: Rect, status: Status, theme: &Theme, scene: &mut Scene) {
        let color = self
            .color
            .or(theme.widgets.icon.color)
            .unwrap_or(theme.on_surface)
            .fade(status.opacity);
        let size = self.resolved_size(Some(theme));
        // The 24×24 grid scaled to the real size, centred in the box — and turned
        // round, if the icon carries a direction and the reading order is right to left.
        let ox = bounds.x + (bounds.width - size) * 0.5;
        let oy = bounds.y + (bounds.height - size) * 0.5;
        // Where the animation has got to, or the one mark this icon is.
        let drawn = match self.morph {
            Some((pair, _)) => pair.at(status.value),
            None => self.icon,
        };
        let path = drawn.placed(size, ox, oy, theme.direction);
        scene.fill_path(&path, color);
    }

    fn on_click(&self) -> Option<Msg> {
        None
    }

    /// Only a morphing icon has an animated value. A plain one answers `None`, so nothing
    /// that was here before this existed acquired an animation.
    fn anim_target(&self) -> Option<f32> {
        self.morph.map(|(_, target)| target)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::icons::Icons;
    use frus_core::Primitive;

    fn paint_icon(icon: Icon) -> Vec<Primitive> {
        let mut scene = Scene::new();
        Widget::<()>::paint(
            &icon,
            Rect::new(0.0, 0.0, 24.0, 24.0),
            Status::default(),
            &Theme::default(),
            &mut scene,
        );
        scene.primitives().to_vec()
    }

    #[test]
    fn paints_a_single_filled_path() {
        let prims = paint_icon(Icon::new(Icons::STAR));
        assert_eq!(prims.len(), 1);
        assert!(matches!(
            prims[0],
            Primitive::Path {
                fill: Some(_),
                stroke: None,
                ..
            }
        ));
    }

    #[test]
    fn color_override_beats_theme() {
        let prims = paint_icon(Icon::new(Icons::FAVORITE).color(Color::rgb(1.0, 0.0, 0.0)));
        match &prims[0] {
            Primitive::Path { fill: Some(c), .. } => {
                assert_eq!(c.r, 1.0);
                assert_eq!(c.g, 0.0);
            }
            _ => panic!("expected a filled path"),
        }
    }

    /// **The drawing follows the animated value, not the flag.** A morphing icon painted
    /// at half its animation is neither of its ends — which is the whole difference
    /// between this and swapping one icon for another.
    #[test]
    fn a_morphing_icon_is_painted_where_the_animation_has_got_to() {
        let paint_at = |value: f32| {
            let mut scene = Scene::new();
            let status = Status {
                value,
                ..Status::default()
            };
            Widget::<()>::paint(
                &Icon::animated(crate::AnimatedIcons::MENU_CLOSE, true),
                Rect::new(0.0, 0.0, 24.0, 24.0),
                status,
                &Theme::default(),
                &mut scene,
            );
            match scene.primitives().first() {
                Some(Primitive::Path { path, .. }) => path.clone(),
                other => panic!("expected a filled path, got {other:?}"),
            }
        };
        let (shut, half, open) = (paint_at(0.0), paint_at(0.5), paint_at(1.0));
        assert_ne!(shut.verbs(), half.verbs(), "half way is not the start");
        assert_ne!(half.verbs(), open.verbs(), "and it is not the end either");
        assert_eq!(
            shut.verbs().len(),
            half.verbs().len(),
            "the same shape all the way across"
        );
    }

    /// The end it is heading for is what the runtime is told, and what an isolated frame
    /// draws. A plain icon says nothing, so nothing that existed before morphs did
    /// acquired an animation.
    #[test]
    fn only_a_morphing_icon_asks_the_runtime_for_a_value() {
        let open = Icon::animated(crate::AnimatedIcons::MENU_CLOSE, true);
        let shut = Icon::animated(crate::AnimatedIcons::MENU_CLOSE, false);
        assert_eq!(Widget::<()>::anim_target(&open), Some(1.0));
        assert_eq!(Widget::<()>::anim_target(&shut), Some(0.0));
        assert_eq!(Widget::<()>::anim_target(&Icon::new(Icons::STAR)), None);
    }

    #[test]
    fn size_drives_the_layout_box() {
        let icon = Icon::new(Icons::CHECK).size(40.0);
        let style = Widget::<()>::style(&icon);
        assert_eq!(style.width, Dimension::Length(40.0));
        assert_eq!(style.height, Dimension::Length(40.0));
    }
}
