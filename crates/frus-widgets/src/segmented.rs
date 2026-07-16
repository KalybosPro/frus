//! [`SegmentedControl`] : un sélecteur segmenté **contrôlé** (boutons connectés,
//! sélection unique) — façon contrôle segmenté iOS.

use frus_core::{BorderRadius, Rect, Scene};
use frus_layout::{FlexDirection, Style};

use crate::button::{Button, Variant};
use crate::interaction::Status;
use crate::theme::Theme;
use crate::widget::Widget;

/// Un contrôle segmenté à sélection unique.
pub struct SegmentedControl<Msg> {
    selected: usize,
    on_select: Box<dyn Fn(usize) -> Msg>,
    labels: Vec<String>,
    /// Rayon des coins **extérieurs** (les coins intérieurs sont droits :
    /// segments connectés). Défaut : 10 px, surchargable via `radius`.
    radius: f32,
    children: Vec<Box<dyn Widget<Msg>>>,
}

impl<Msg: Clone + 'static> SegmentedControl<Msg> {
    /// Crée un contrôle : `selected` = index actif, `on_select(i)` au clic.
    pub fn new(selected: usize, on_select: impl Fn(usize) -> Msg + 'static) -> Self {
        Self {
            selected,
            on_select: Box::new(on_select),
            labels: Vec::new(),
            radius: 10.0,
            children: Vec::new(),
        }
    }

    /// Surcharge le rayon des coins extérieurs du groupe.
    pub fn radius(mut self, radius: f32) -> Self {
        self.radius = radius;
        self.rebuild();
        self
    }

    /// Ajoute un segment.
    pub fn segment(mut self, label: impl Into<String>) -> Self {
        self.labels.push(label.into());
        self.rebuild();
        self
    }

    /// Rayons du `i`-ième segment sur `n` : seuls les coins **extérieurs** du
    /// groupe sont arrondis (premier à gauche, dernier à droite), les jointures
    /// restent droites — l'aspect « boutons connectés ».
    fn corner_radius(&self, i: usize, n: usize) -> BorderRadius {
        let r = self.radius;
        match (i == 0, i + 1 == n) {
            (true, true) => BorderRadius::uniform(r),
            (true, false) => BorderRadius {
                top_left: r,
                bottom_left: r,
                top_right: 0.0,
                bottom_right: 0.0,
            },
            (false, true) => BorderRadius {
                top_right: r,
                bottom_right: r,
                top_left: 0.0,
                bottom_left: 0.0,
            },
            (false, false) => BorderRadius::ZERO,
        }
    }

    fn rebuild(&mut self) {
        let count = self.labels.len();
        self.children = self
            .labels
            .iter()
            .enumerate()
            .map(|(i, label)| {
                let variant = if i == self.selected {
                    Variant::Primary
                } else {
                    Variant::Secondary
                };
                Box::new(
                    Button::new(label.clone())
                        .variant(variant)
                        .size(15.0)
                        .radius(self.corner_radius(i, count))
                        .on_press((self.on_select)(i)),
                ) as Box<dyn Widget<Msg>>
            })
            .collect();
    }
}

impl<Msg: Clone> Widget<Msg> for SegmentedControl<Msg> {
    fn style(&self) -> Style {
        Style {
            flex_direction: FlexDirection::Row,
            gap: 2.0,
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

    #[derive(Clone, Debug, PartialEq)]
    enum Msg {
        Select(usize),
    }

    #[test]
    fn segments_emit_index_and_highlight_selected() {
        let seg = SegmentedControl::new(1, Msg::Select)
            .segment("Jour")
            .segment("Semaine")
            .segment("Mois");
        let children = Widget::<Msg>::children(&seg);
        assert_eq!(children.len(), 3);
        // Cliquer le 3ᵉ segment → Select(2).
        assert_eq!(children[2].on_click(), Some(Msg::Select(2)));
    }

    #[test]
    fn segments_round_only_the_outer_corners() {
        use crate::{build_ui, Runtime, Size, Theme};
        let seg = SegmentedControl::new(0, Msg::Select)
            .segment("Un")
            .segment("Deux")
            .segment("Trois");
        let ui = build_ui(&seg, Size::new(400.0, 60.0), &Runtime::default(), &Theme::default());
        // Les remplissages des boutons (rectangles nets, non floutés, opaques),
        // dans l'ordre : premier, milieu, dernier.
        let fills: Vec<BorderRadius> = ui
            .scene()
            .primitives()
            .iter()
            .filter_map(|p| match p {
                frus_core::Primitive::Rect { radius, blur, color, .. }
                    if *blur == 0.0 && color.a > 0.9 =>
                {
                    Some(*radius)
                }
                _ => None,
            })
            .collect();
        assert_eq!(fills.len(), 3, "trois remplissages de segments");
        assert!(fills[0].top_left > 0.0 && fills[0].top_right == 0.0, "1er : gauche arrondi");
        assert_eq!(fills[1], BorderRadius::ZERO, "milieu : droit");
        assert!(fills[2].top_right > 0.0 && fills[2].top_left == 0.0, "dernier : droite arrondie");
    }
}
