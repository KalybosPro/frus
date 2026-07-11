//! [`NavScaffold`] : l'ossature de navigation **adaptative**. Selon la
//! [`SizeClass`], elle place la navigation principale en **rail vertical**
//! (Medium/Expanded) ou en **barre basse** (Compact), le corps remplissant le
//! reste. C'est ici que la classe de taille pilote la *structure* de l'écran.

use frus_core::{Rect, Scene, SizeClass};
use frus_layout::{Dimension, FlexDirection, Style};

use crate::flex::Flex;
use crate::interaction::Status;
use crate::navrail::{BottomBar, NavRail};
use crate::theme::Theme;
use crate::widget::Widget;

/// Ossature de navigation adaptative (rail ↔ barre basse selon la taille).
pub struct NavScaffold<Msg> {
    compact: bool,
    selected: usize,
    on_select: Option<Box<dyn Fn(usize) -> Msg>>,
    destinations: Vec<(String, String, Option<u32>)>,
    children: Vec<Box<dyn Widget<Msg>>>,
}

impl<Msg: Clone + 'static> NavScaffold<Msg> {
    /// Crée une ossature : `class` détermine la présentation, `selected` la
    /// destination active, `on_select(i)` est émis au choix d'une destination.
    pub fn new(class: SizeClass, selected: usize, on_select: impl Fn(usize) -> Msg + 'static) -> Self {
        Self {
            compact: class == SizeClass::Compact,
            selected,
            on_select: Some(Box::new(on_select)),
            destinations: Vec::new(),
            children: Vec::new(),
        }
    }

    /// Ajoute une destination (glyphe + libellé). À appeler **avant** [`body`].
    ///
    /// [`body`]: NavScaffold::body
    pub fn destination(mut self, icon: impl Into<String>, label: impl Into<String>) -> Self {
        self.destinations.push((icon.into(), label.into(), None));
        self
    }

    /// Ajoute un compteur de notifications à la **dernière** destination.
    pub fn badge(mut self, count: u32) -> Self {
        if let Some(last) = self.destinations.last_mut() {
            last.2 = Some(count);
        }
        self
    }

    /// Définit le corps et **finalise** l'ossature (appeler en dernier). Le corps
    /// remplit l'espace restant à côté (ou au-dessus) de la navigation.
    pub fn body(mut self, body: impl Widget<Msg> + 'static) -> Self {
        let on_select = self.on_select.take().expect("body appelé une seule fois");
        let destinations = std::mem::take(&mut self.destinations);
        let body_pane: Box<dyn Widget<Msg>> = Box::new(Flex::column().flex(1.0).child(body));

        // Un seul des deux bras s'exécute : `on_select` n'est déplacé qu'une fois.
        let nav: Box<dyn Widget<Msg>> = if self.compact {
            let mut bar = BottomBar::new(self.selected, on_select);
            for (icon, label, badge) in destinations {
                bar = bar.item(icon, label);
                if let Some(count) = badge {
                    bar = bar.badge(count);
                }
            }
            Box::new(bar)
        } else {
            let mut rail = NavRail::new(self.selected, on_select);
            for (icon, label, badge) in destinations {
                rail = rail.item(icon, label);
                if let Some(count) = badge {
                    rail = rail.badge(count);
                }
            }
            Box::new(rail)
        };

        // Compact : corps au-dessus, barre en bas. Sinon : rail à gauche, corps à droite.
        self.children = if self.compact {
            vec![body_pane, nav]
        } else {
            vec![nav, body_pane]
        };
        self
    }
}

impl<Msg: Clone> Widget<Msg> for NavScaffold<Msg> {
    fn style(&self) -> Style {
        Style {
            width: Dimension::Percent(1.0),
            height: Dimension::Percent(1.0),
            flex_direction: if self.compact {
                FlexDirection::Column
            } else {
                FlexDirection::Row
            },
            ..Default::default()
        }
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &self.children
    }

    fn paint(&self, _bounds: Rect, _status: Status, _theme: &Theme, _scene: &mut Scene) {}

    fn on_click(&self) -> Option<Msg> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Container;

    #[derive(Clone, Debug, PartialEq)]
    enum Msg {
        Go(usize),
    }

    fn scaffold(class: SizeClass) -> NavScaffold<Msg> {
        NavScaffold::new(class, 0, Msg::Go)
            .destination("H", "Home")
            .destination("S", "Settings")
            .body(Container::new())
    }

    #[test]
    fn compact_puts_body_first_then_bottom_bar_in_a_column() {
        let s = scaffold(SizeClass::Compact);
        assert_eq!(Widget::<Msg>::style(&s).flex_direction, FlexDirection::Column);
        // [corps, barre] : la navigation est le dernier enfant (en bas).
        assert_eq!(Widget::<Msg>::children(&s).len(), 2);
    }

    #[test]
    fn expanded_puts_rail_first_then_body_in_a_row() {
        let s = scaffold(SizeClass::Expanded);
        assert_eq!(Widget::<Msg>::style(&s).flex_direction, FlexDirection::Row);
        // Le rail (1er enfant) a une largeur fixe ; le corps prend le reste.
        let children = Widget::<Msg>::children(&s);
        assert_eq!(
            Widget::<Msg>::style(&*children[0]).width,
            Dimension::Length(crate::navrail::RAIL_WIDTH)
        );
    }
}
