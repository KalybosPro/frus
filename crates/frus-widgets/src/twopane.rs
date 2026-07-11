//! [`TwoPane`] : le patron **maître-détail** responsive. En `Expanded`, la liste
//! et le détail sont **côte à côte** ; sinon, un **seul panneau** est montré (la
//! liste, ou le détail si `show_detail` — l'app décide en naviguant).

use frus_core::{Rect, Scene, SizeClass};
use frus_layout::{Dimension, FlexDirection, Style};

use crate::flex::Flex;
use crate::interaction::Status;
use crate::theme::Theme;
use crate::widget::Widget;

/// Un agencement maître-détail adaptatif.
pub struct TwoPane<Msg> {
    class: SizeClass,
    ratio: f32,
    show_detail: bool,
    list: Option<Box<dyn Widget<Msg>>>,
    row: bool,
    children: Vec<Box<dyn Widget<Msg>>>,
}

impl<Msg: Clone + 'static> TwoPane<Msg> {
    /// Crée un agencement pour la classe `class` (ratio liste par défaut 0.38).
    pub fn new(class: SizeClass) -> Self {
        Self {
            class,
            ratio: 0.38,
            show_detail: false,
            list: None,
            row: false,
            children: Vec::new(),
        }
    }

    /// Fraction de largeur allouée à la liste en mode côte à côte (`0.1..=0.9`).
    /// À définir **avant** [`detail`](TwoPane::detail).
    pub fn ratio(mut self, ratio: f32) -> Self {
        self.ratio = ratio.clamp(0.1, 0.9);
        self
    }

    /// En panneau unique, montre le **détail** plutôt que la liste (l'app le met
    /// à `true` quand elle navigue vers un élément).
    pub fn show_detail(mut self, show: bool) -> Self {
        self.show_detail = show;
        self
    }

    /// Définit la **liste** (panneau maître). À appeler avant [`detail`](TwoPane::detail).
    pub fn list(mut self, list: impl Widget<Msg> + 'static) -> Self {
        self.list = Some(Box::new(list));
        self
    }

    /// Définit le **détail** et **finalise** l'agencement (appeler en dernier).
    pub fn detail(mut self, detail: impl Widget<Msg> + 'static) -> Self {
        let detail: Box<dyn Widget<Msg>> = Box::new(detail);
        if self.class == SizeClass::Expanded {
            // Côte à côte : largeurs proportionnelles via flex_grow.
            let list = self.list.take().expect("list() avant detail() en mode côte à côte");
            self.children = vec![
                Box::new(Flex::column().flex(self.ratio).child(list)),
                Box::new(Flex::column().flex(1.0 - self.ratio).child(detail)),
            ];
            self.row = true;
        } else {
            // Panneau unique : détail si demandé, sinon liste.
            let single = if self.show_detail {
                detail
            } else {
                self.list.take().expect("list() ou show_detail requis en panneau unique")
            };
            self.children = vec![Box::new(Flex::column().flex(1.0).child(single))];
            self.row = false;
        }
        self
    }
}

impl<Msg: Clone> Widget<Msg> for TwoPane<Msg> {
    fn style(&self) -> Style {
        Style {
            width: Dimension::Percent(1.0),
            height: Dimension::Percent(1.0),
            flex_direction: if self.row {
                FlexDirection::Row
            } else {
                FlexDirection::Column
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

    #[test]
    fn expanded_shows_both_panes_side_by_side() {
        let tp = TwoPane::<()>::new(SizeClass::Expanded)
            .ratio(0.4)
            .list(Container::new())
            .detail(Container::new());
        assert_eq!(Widget::<()>::style(&tp).flex_direction, FlexDirection::Row);
        assert_eq!(Widget::<()>::children(&tp).len(), 2);
        // Panneau liste = flex 0.4, détail = flex 0.6.
        let panes = Widget::<()>::children(&tp);
        assert_eq!(Widget::<()>::style(&*panes[0]).flex_grow, 0.4);
        assert!((Widget::<()>::style(&*panes[1]).flex_grow - 0.6).abs() < 1e-6);
    }

    #[test]
    fn compact_shows_a_single_pane() {
        let list_only = TwoPane::<()>::new(SizeClass::Compact)
            .list(Container::new())
            .detail(Container::new());
        assert_eq!(Widget::<()>::style(&list_only).flex_direction, FlexDirection::Column);
        assert_eq!(Widget::<()>::children(&list_only).len(), 1);

        // Avec show_detail, c'est le détail qui occupe l'unique panneau.
        let detail_shown = TwoPane::<()>::new(SizeClass::Compact)
            .show_detail(true)
            .list(Container::new())
            .detail(Container::new());
        assert_eq!(Widget::<()>::children(&detail_shown).len(), 1);
    }
}
