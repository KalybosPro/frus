//! [`FittedBox`] : met son enfant à l'échelle pour l'**ajuster** à sa boîte selon un
//! [`BoxFit`] — et, à la différence de [`crate::Transform`], l'échelle découle de la
//! **mise en page** (la taille de la boîte), façon `FittedBox` de Flutter.

use frus_core::{BoxFit, Rect, Scene};
use frus_layout::{Dimension, Style};

use crate::interaction::Status;
use crate::theme::Theme;
use crate::widget::Widget;

/// Met son enfant à l'échelle pour l'**ajuster** à sa propre boîte selon un
/// [`BoxFit`] (comme `object-fit` en CSS), puis le centre. L'enfant est mesuré à sa
/// taille **naturelle** ; le facteur d'échelle en découle — d'où l'effet sur la mise
/// en page (contrairement à `Transform`, où l'échelle est fixée à la main).
///
/// Idéal pour qu'un contenu de taille intrinsèque (texte, icône, dessin) **remplisse**
/// ou **tienne** dans un cadre donné sans calcul manuel. La boîte a besoin d'une
/// taille (comme [`crate::Scroll`]) : `width`/`height` fixes ou `flex`.
///
/// ```ignore
/// FittedBox::new(BoxFit::Contain).width(120.0).height(40.0).child(Text::new("Big"))
/// ```
pub struct FittedBox<Msg> {
    fit: BoxFit,
    width: Dimension,
    height: Dimension,
    flex_grow: f32,
    children: Vec<Box<dyn Widget<Msg>>>,
}

impl<Msg> FittedBox<Msg> {
    /// Ajuste l'enfant selon `fit` (défaut usuel : [`BoxFit::Contain`]).
    pub fn new(fit: BoxFit) -> Self {
        Self {
            fit,
            width: Dimension::Auto,
            height: Dimension::Auto,
            flex_grow: 0.0,
            children: Vec::new(),
        }
    }

    /// Largeur fixe de la boîte (px logiques).
    pub fn width(mut self, width: f32) -> Self {
        self.width = Dimension::Length(width);
        self
    }

    /// Hauteur fixe de la boîte (px logiques).
    pub fn height(mut self, height: f32) -> Self {
        self.height = Dimension::Length(height);
        self
    }

    /// Facteur d'expansion flex sur l'axe principal du parent.
    pub fn flex(mut self, grow: f32) -> Self {
        self.flex_grow = grow;
        self
    }

    /// Définit l'enfant ajusté.
    pub fn child(mut self, child: impl Widget<Msg> + 'static) -> Self {
        self.children.clear();
        self.children.push(Box::new(child));
        self
    }
}

impl<Msg: Clone> Widget<Msg> for FittedBox<Msg> {
    fn style(&self) -> Style {
        Style {
            width: self.width,
            height: self.height,
            flex_grow: self.flex_grow,
            ..Default::default()
        }
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &self.children
    }

    fn paint(&self, _bounds: Rect, _status: Status, _theme: &Theme, _scene: &mut Scene) {
        // Widget d'ajustement pur : aucune décoration propre.
    }

    fn on_click(&self) -> Option<Msg> {
        None
    }

    fn fitted(&self) -> Option<BoxFit> {
        Some(self.fit)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Container, Flex};
    use frus_core::{Color, Primitive, Size};

    /// `BoxFit::Fill` étire l'enfant pour **remplir** la boîte : la matrice du calque
    /// porte l'échelle par axe (200/40 = 5 en x, 100/20 = 5 en y ici → carré : 5,5).
    #[test]
    fn fill_scales_child_to_the_box() {
        let root = Flex::<()>::column().width(200.0).child(
            FittedBox::new(BoxFit::Fill)
                .width(200.0)
                .height(100.0)
                .child(Container::new().width(40.0).height(20.0).color(Color::rgb(0.3, 0.3, 0.3))),
        );
        let rt = crate::runtime::Runtime::default();
        let theme = crate::Theme::dark();
        let ui = crate::ui::build_ui(&root, Size::new(200.0, 400.0), &rt, &theme);
        let m = ui
            .scene()
            .primitives()
            .iter()
            .find_map(|p| match p {
                Primitive::Layer { transform: Some(t), .. } => Some(t.affine),
                _ => None,
            })
            .expect("un calque ajusté");
        assert!(
            (m.m[0] - 5.0).abs() < 1e-2 && (m.m[3] - 5.0).abs() < 1e-2,
            "échelle Fill 5×5 : {:?}",
            m.m
        );
    }

    /// `BoxFit::Contain` conserve l'aspect : le plus petit facteur qui tient. Enfant
    /// 40×20 dans 200×100 → min(5, 5) = 5 (carré), reste centré.
    #[test]
    fn contain_preserves_aspect() {
        let root = Flex::<()>::column().width(200.0).child(
            FittedBox::new(BoxFit::Contain)
                .width(200.0)
                .height(100.0)
                .child(Container::new().width(40.0).height(40.0).color(Color::rgb(0.3, 0.3, 0.3))),
        );
        let rt = crate::runtime::Runtime::default();
        let theme = crate::Theme::dark();
        let ui = crate::ui::build_ui(&root, Size::new(200.0, 400.0), &rt, &theme);
        let m = ui
            .scene()
            .primitives()
            .iter()
            .find_map(|p| match p {
                Primitive::Layer { transform: Some(t), .. } => Some(t.affine),
                _ => None,
            })
            .expect("un calque ajusté");
        // Enfant carré 40×40 dans 200×100 → min(5, 2.5) = 2.5, uniforme.
        assert!(
            (m.m[0] - 2.5).abs() < 1e-2 && (m.m[3] - 2.5).abs() < 1e-2,
            "échelle Contain 2.5×2.5 : {:?}",
            m.m
        );
    }
}
