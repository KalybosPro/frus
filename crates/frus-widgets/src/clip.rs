//! [`ClipRRect`] et [`ClipOval`] : découpent leur enfant à une **forme** (coins
//! arrondis, ellipse) à la peinture, façon `ClipRRect` / `ClipOval` de Flutter.

use frus_core::{ClipShape, Rect, Scene};
use frus_layout::Style;

use crate::interaction::Status;
use crate::theme::Theme;
use crate::widget::Widget;

/// Découpe son enfant à un **rectangle à coins arrondis**. Le sous-arbre est peint
/// dans un calque dont la forme gomme ce qui déborde des coins (bords anticrénelés) —
/// la brique d'une vignette, d'un avatar carré-arrondi, d'une carte à coins doux dont
/// le contenu (image, dégradé…) épouse exactement l'arrondi.
///
/// Passe-plat en mise en page : la boîte prend la taille que le parent lui donne (comme
/// son enfant), et l'arrondi est **inscrit** dans cette boîte. Le rayon est borné à la
/// demi-plus-petite dimension (au-delà, les coins se rejoignent — un stade).
///
/// ```ignore
/// ClipRRect::new(12.0).child(Image::asset("photo.png"))
/// ```
pub struct ClipRRect<Msg> {
    radius: f32,
    children: Vec<Box<dyn Widget<Msg>>>,
}

impl<Msg> ClipRRect<Msg> {
    /// Découpe l'enfant à un rectangle arrondi de `radius` px logiques (uniforme sur
    /// les quatre coins).
    pub fn new(radius: f32) -> Self {
        Self { radius, children: Vec::new() }
    }

    /// Définit l'enfant découpé.
    pub fn child(mut self, child: impl Widget<Msg> + 'static) -> Self {
        self.children.clear();
        self.children.push(Box::new(child));
        self
    }
}

impl<Msg: Clone> Widget<Msg> for ClipRRect<Msg> {
    fn style(&self) -> Style {
        // Passe-plat : la boîte prend sa taille du contexte comme l'enfant ; la
        // découpe n'agit qu'à la peinture (voir `clip_shape`).
        Style::default()
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &self.children
    }

    fn paint(&self, _bounds: Rect, _status: Status, _theme: &Theme, _scene: &mut Scene) {
        // Widget de découpe pur : aucune décoration propre.
    }

    fn on_click(&self) -> Option<Msg> {
        None
    }

    fn clip_shape(&self) -> Option<ClipShape> {
        Some(ClipShape::RRect(self.radius))
    }
}

/// Découpe son enfant à une **ellipse** inscrite dans sa boîte (un cercle si la boîte
/// est carrée) : la brique d'un avatar rond, d'une pastille, d'une jauge circulaire
/// dont le contenu est rogné au disque. Mêmes règles de mise en page que
/// [`ClipRRect`] (passe-plat, forme inscrite dans la boîte).
///
/// ```ignore
/// ClipOval::new().child(Image::asset("avatar.png"))
/// ```
pub struct ClipOval<Msg> {
    children: Vec<Box<dyn Widget<Msg>>>,
}

impl<Msg> ClipOval<Msg> {
    /// Découpe l'enfant à l'ellipse inscrite dans la boîte.
    pub fn new() -> Self {
        Self { children: Vec::new() }
    }

    /// Définit l'enfant découpé.
    pub fn child(mut self, child: impl Widget<Msg> + 'static) -> Self {
        self.children.clear();
        self.children.push(Box::new(child));
        self
    }
}

impl<Msg> Default for ClipOval<Msg> {
    fn default() -> Self {
        Self::new()
    }
}

impl<Msg: Clone> Widget<Msg> for ClipOval<Msg> {
    fn style(&self) -> Style {
        Style::default()
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &self.children
    }

    fn paint(&self, _bounds: Rect, _status: Status, _theme: &Theme, _scene: &mut Scene) {}

    fn on_click(&self) -> Option<Msg> {
        None
    }

    fn clip_shape(&self) -> Option<ClipShape> {
        Some(ClipShape::Oval)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Container;
    use frus_core::{Color, Primitive, Size};

    /// Enveloppe la scène d'un `ClipRRect` : un calque à forme `RRect` est émis, et
    /// le fond de l'enfant est peint **dedans** (dans les primitives du calque).
    #[test]
    fn clip_rrect_wraps_child_in_a_rounded_layer() {
        let red = Color::rgb(1.0, 0.0, 0.0);
        let root = crate::Flex::<()>::column().width(100.0).child(
            ClipRRect::new(8.0).child(Container::new().width(40.0).height(40.0).color(red)),
        );
        let rt = crate::runtime::Runtime::default();
        let theme = crate::Theme::dark();
        let ui = crate::ui::build_ui(&root, Size::new(100.0, 200.0), &rt, &theme);
        let layer = ui
            .scene()
            .primitives()
            .iter()
            .find_map(|p| match p {
                Primitive::Layer { clip_shape, primitives, .. } => Some((*clip_shape, primitives.clone())),
                _ => None,
            })
            .expect("un calque de découpe");
        assert_eq!(layer.0, ClipShape::RRect(8.0), "forme arrondie de rayon 8");
        assert!(
            layer.1.iter().any(|p| matches!(p, Primitive::Rect { color, .. } if color.r > 0.5)),
            "le fond rouge de l'enfant est peint dans le calque"
        );
    }

    /// `ClipOval` émet un calque à forme `Oval`.
    #[test]
    fn clip_oval_emits_an_oval_layer() {
        let blue = Color::rgb(0.0, 0.0, 1.0);
        let root = crate::Flex::<()>::column().width(100.0).child(
            ClipOval::new().child(Container::new().width(40.0).height(40.0).color(blue)),
        );
        let rt = crate::runtime::Runtime::default();
        let theme = crate::Theme::dark();
        let ui = crate::ui::build_ui(&root, Size::new(100.0, 200.0), &rt, &theme);
        let shape = ui.scene().primitives().iter().find_map(|p| match p {
            Primitive::Layer { clip_shape, .. } => Some(*clip_shape),
            _ => None,
        });
        assert_eq!(shape, Some(ClipShape::Oval), "forme ellipse");
    }

    /// La découpe est **passe-plat** en mise en page : un frère placé après un enfant
    /// découpé garde sa position (le `ClipRRect` n'agrandit pas sa boîte).
    #[test]
    fn clip_is_layout_passthrough() {
        let red = Color::rgb(1.0, 0.0, 0.0);
        let green = Color::rgb(0.0, 1.0, 0.0);
        let root = crate::Flex::<()>::column()
            .width(100.0)
            .child(ClipRRect::new(6.0).child(Container::new().height(20.0).color(red)))
            .child(Container::new().height(20.0).color(green));
        let rt = crate::runtime::Runtime::default();
        let theme = crate::Theme::dark();
        let ui = crate::ui::build_ui(&root, Size::new(100.0, 200.0), &rt, &theme);
        // Le 2e enfant suit à y = 20 (l'enfant découpé occupe 20px de haut).
        let green_y = ui
            .scene()
            .primitives()
            .iter()
            .flat_map(|p| match p {
                Primitive::Layer { primitives, .. } => primitives.clone(),
                other => vec![other.clone()],
            })
            .find_map(|p| match p {
                Primitive::Rect { rect, color, .. } if color.g > 0.5 && color.r < 0.5 => Some(rect.y),
                _ => None,
            })
            .expect("le fond vert du 2e enfant");
        assert!((green_y - 20.0).abs() < 0.5, "frère à sa place layout : y = {green_y}");
    }
}
