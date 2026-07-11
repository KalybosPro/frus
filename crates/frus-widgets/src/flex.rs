//! [`Flex`] : un conteneur qui dispose ses enfants en rangée ou en colonne,
//! avec répartition (justify) et alignement (align).

use frus_core::{Insets, Rect, Scene};
use frus_layout::{Align, Dimension, FlexDirection, Justify, Style};

use crate::interaction::Status;
use crate::theme::Theme;
use crate::widget::Widget;

/// Un conteneur flex (rangée ou colonne). Ne peint aucune décoration propre.
pub struct Flex<Msg> {
    direction: FlexDirection,
    width: Dimension,
    height: Dimension,
    flex_grow: f32,
    justify: Justify,
    align: Align,
    padding: Insets,
    gap: f32,
    wrap: bool,
    children: Vec<Box<dyn Widget<Msg>>>,
}

impl<Msg> Flex<Msg> {
    /// Conteneur disposant ses enfants horizontalement.
    pub fn row() -> Self {
        Self::with_direction(FlexDirection::Row)
    }

    /// Conteneur disposant ses enfants verticalement.
    pub fn column() -> Self {
        Self::with_direction(FlexDirection::Column)
    }

    fn with_direction(direction: FlexDirection) -> Self {
        Self {
            direction,
            width: Dimension::Auto,
            height: Dimension::Auto,
            flex_grow: 0.0,
            justify: Justify::Start,
            align: Align::Stretch,
            padding: Insets::ZERO,
            gap: 0.0,
            wrap: false,
            children: Vec::new(),
        }
    }

    /// Fixe la largeur, en pixels logiques.
    pub fn width(mut self, width: f32) -> Self {
        self.width = Dimension::Length(width);
        self
    }

    /// Fixe la hauteur, en pixels logiques.
    pub fn height(mut self, height: f32) -> Self {
        self.height = Dimension::Length(height);
        self
    }

    /// Facteur d'expansion flex sur l'axe principal du parent.
    pub fn flex(mut self, grow: f32) -> Self {
        self.flex_grow = grow;
        self
    }

    /// Répartition des enfants sur l'axe principal.
    pub fn justify(mut self, justify: Justify) -> Self {
        self.justify = justify;
        self
    }

    /// Alignement des enfants sur l'axe croisé.
    pub fn align(mut self, align: Align) -> Self {
        self.align = align;
        self
    }

    /// Marge intérieure uniforme, en pixels logiques.
    pub fn padding(mut self, padding: f32) -> Self {
        self.padding = Insets::uniform(padding);
        self
    }

    /// Marge intérieure par côté (haut, droite, bas, gauche).
    pub fn padding_each(mut self, top: f32, right: f32, bottom: f32, left: f32) -> Self {
        self.padding = Insets::new(top, right, bottom, left);
        self
    }

    /// Espacement entre enfants, en pixels logiques.
    pub fn gap(mut self, gap: f32) -> Self {
        self.gap = gap;
        self
    }

    /// Active le **passage à la ligne** : les enfants qui débordent l'axe
    /// principal repassent sur une nouvelle ligne (reflow responsive). Voir aussi
    /// [`Wrap`] comme point d'entrée nommé.
    pub fn wrap(mut self) -> Self {
        self.wrap = true;
        self
    }

    /// Ajoute un enfant.
    pub fn child(mut self, child: impl Widget<Msg> + 'static) -> Self {
        self.children.push(Box::new(child));
        self
    }
}

impl<Msg: Clone> Widget<Msg> for Flex<Msg> {
    fn style(&self) -> Style {
        Style {
            width: self.width,
            height: self.height,
            flex_grow: self.flex_grow,
            flex_direction: self.direction,
            justify: self.justify,
            align: self.align,
            padding: self.padding,
            gap: self.gap,
            flex_wrap: self.wrap,
            grid_columns: None,
        }
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &self.children
    }

    fn paint(&self, _bounds: Rect, _status: Status, _theme: &Theme, _scene: &mut Scene) {
        // Un conteneur flex est transparent : pas de décoration propre.
    }

    fn on_click(&self) -> Option<Msg> {
        None
    }
}

/// Point d'entrée nommé pour une rangée **qui passe à la ligne** (flex-wrap).
///
/// `Wrap::new().gap(8.0).child(a).child(b)…` — les enfants s'écoulent sur
/// plusieurs lignes selon la largeur disponible, sans breakpoint. Renvoie un
/// [`Flex`] (rangée avec `wrap` activé), donc tous ses réglages restent dispo.
pub struct Wrap;

impl Wrap {
    /// Rangée avec passage à la ligne activé.
    #[allow(clippy::new_ret_no_self)]
    pub fn new<Msg>() -> Flex<Msg> {
        Flex::row().wrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wrap_sets_flex_wrap_in_style() {
        let plain = Flex::<()>::row();
        assert!(!Widget::<()>::style(&plain).flex_wrap);

        let wrapped = Wrap::new::<()>().child(Flex::<()>::row());
        assert!(Widget::<()>::style(&wrapped).flex_wrap);
    }
}
