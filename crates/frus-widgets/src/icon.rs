//! [`Icon`]: displays a vector icon from the bundled set ([`Icons`]), scaled and
//! coloured to the theme. It is the first consumer of the vector paths
//! ([`frus_core::Path`]) on the widget side.

use frus_core::{Color, Rect, Scene};
use frus_layout::{Dimension, Style};

use crate::icons::{IconData, GRID};
use crate::interaction::Status;
use crate::theme::Theme;
use crate::widget::Widget;

/// A vector icon. Size and colour can both be customised; they default to `24` px
/// and the theme's foreground colour (`on_surface`).
pub struct Icon {
    icon: IconData,
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
        let path = self.icon.placed(size, ox, oy, theme.direction);
        scene.fill_path(&path, color);
    }

    fn on_click(&self) -> Option<Msg> {
        None
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

    #[test]
    fn size_drives_the_layout_box() {
        let icon = Icon::new(Icons::CHECK).size(40.0);
        let style = Widget::<()>::style(&icon);
        assert_eq!(style.width, Dimension::Length(40.0));
        assert_eq!(style.height, Dimension::Length(40.0));
    }
}
