//! [`Icon`]: displays a vector icon from the bundled set ([`Icons`]), scaled and
//! coloured to the theme. It is the first consumer of the vector paths
//! ([`frus_core::Path`]) on the widget side.

use frus_core::{Color, Rect, Scene};
use frus_layout::{Dimension, Style};

use crate::icons::Icons;
use crate::interaction::Status;
use crate::theme::Theme;
use crate::widget::Widget;

/// The icon grid's reference size; see [`crate::icons`].
const GRID: f32 = 24.0;

/// A vector icon. Size and colour can both be customised; they default to `24` px
/// and the theme's foreground colour (`on_surface`).
pub struct Icon {
    name: Icons,
    size: f32,
    color: Option<Color>,
}

impl Icon {
    /// A `24` px icon, in the theme's colour.
    pub fn new(name: Icons) -> Self {
        Self {
            name,
            size: GRID,
            color: None,
        }
    }

    /// Sets the size, that is, the square's side, in logical pixels.
    pub fn size(mut self, size: f32) -> Self {
        self.size = size;
        self
    }

    /// Forces the colour; the theme's `on_surface` otherwise.
    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }
}

impl<Msg> Widget<Msg> for Icon {
    fn style(&self) -> Style {
        Style {
            width: Dimension::Length(self.size),
            height: Dimension::Length(self.size),
            ..Default::default()
        }
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &[]
    }

    fn paint(&self, bounds: Rect, status: Status, theme: &Theme, scene: &mut Scene) {
        let color = self.color.unwrap_or(theme.on_surface).fade(status.opacity);
        // The 24×24 grid scaled to the real size, centred in the box.
        let scale = self.size / GRID;
        let ox = bounds.x + (bounds.width - self.size) * 0.5;
        let oy = bounds.y + (bounds.height - self.size) * 0.5;
        let path = self.name.path().scaled(scale).translated(ox, oy);
        scene.fill_path(&path, color);
    }

    fn on_click(&self) -> Option<Msg> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
        let prims = paint_icon(Icon::new(Icons::Star));
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
        let prims = paint_icon(Icon::new(Icons::Heart).color(Color::rgb(1.0, 0.0, 0.0)));
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
        let icon = Icon::new(Icons::Check).size(40.0);
        let style = Widget::<()>::style(&icon);
        assert_eq!(style.width, Dimension::Length(40.0));
        assert_eq!(style.height, Dimension::Length(40.0));
    }
}
