//! [`Icon`] : affiche une icône vectorielle du jeu embarqué ([`IconName`]),
//! mise à l'échelle et colorée au thème. C'est le premier consommateur des
//! chemins vectoriels ([`frus_core::Path`]) côté widgets.

use frus_core::{Color, Rect, Scene};
use frus_layout::{Dimension, Style};

use crate::icons::IconName;
use crate::interaction::Status;
use crate::theme::Theme;
use crate::widget::Widget;

/// Taille de référence de la grille des icônes (voir [`crate::icons`]).
const GRID: f32 = 24.0;

/// Une icône vectorielle. Taille et couleur sont personnalisables ; par défaut
/// `24` px et la couleur de premier plan du thème (`on_surface`).
pub struct Icon {
    name: IconName,
    size: f32,
    color: Option<Color>,
}

impl Icon {
    /// Une icône `24` px, couleur du thème.
    pub fn new(name: IconName) -> Self {
        Self {
            name,
            size: GRID,
            color: None,
        }
    }

    /// Fixe la taille (côté du carré), en pixels logiques.
    pub fn size(mut self, size: f32) -> Self {
        self.size = size;
        self
    }

    /// Force la couleur (sinon `on_surface` du thème).
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
        // Grille 24×24 → taille réelle, centrée dans la boîte.
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
        let prims = paint_icon(Icon::new(IconName::Star));
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
        let prims = paint_icon(Icon::new(IconName::Heart).color(Color::rgb(1.0, 0.0, 0.0)));
        match &prims[0] {
            Primitive::Path { fill: Some(c), .. } => {
                assert_eq!(c.r, 1.0);
                assert_eq!(c.g, 0.0);
            }
            _ => panic!("attendu un chemin rempli"),
        }
    }

    #[test]
    fn size_drives_the_layout_box() {
        let icon = Icon::new(IconName::Check).size(40.0);
        let style = Widget::<()>::style(&icon);
        assert_eq!(style.width, Dimension::Length(40.0));
        assert_eq!(style.height, Dimension::Length(40.0));
    }
}
