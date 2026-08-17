//! [`RadioGroup`]: a group of radio buttons, with one option selected.

use frus_core::{Color, Point, Rect, Scene};
use frus_layout::{Dimension, FlexDirection, Style};

use crate::disabled::disabled_content;
use crate::interaction::Status;
use crate::theme::Theme;
use crate::widget::Widget;

const DOT: f32 = 20.0;
const GAP: f32 = 10.0;

/// One radio option, internal to the group.
struct RadioOption<Msg> {
    label: String,
    selected: bool,
    size: f32,
    /// The group's availability, handed down. An option that stayed live under a disabled
    /// group would be the whole group, since a group is only ever its options.
    enabled: bool,
    on_click: Option<Msg>,
}

impl<Msg: Clone> Widget<Msg> for RadioOption<Msg> {
    fn style(&self) -> Style {
        let line = frus_text::line_height(self.size).max(DOT);
        let label_w = frus_text::measure(&self.label, self.size).width;
        Style {
            width: Dimension::Length((DOT + GAP + label_w).ceil()),
            height: Dimension::Length(line.ceil()),
            ..Default::default()
        }
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &[]
    }

    fn paint(&self, bounds: Rect, status: Status, theme: &Theme, scene: &mut Scene) {
        let o = status.opacity;
        let cy = bounds.y + (bounds.height - DOT) * 0.5;
        let outer = Rect::new(bounds.x, cy, DOT, DOT);
        // A radio has no container: the ring and the dot *are* the control, so both take
        // the content opacity — the reference disables its fill at 38 % whether the option
        // is the chosen one or not.
        let (ring, dot, label) = if self.enabled {
            (
                if self.selected {
                    theme.primary
                } else {
                    theme.border
                },
                theme.primary,
                theme.on_surface,
            )
        } else {
            let dead = disabled_content(theme);
            (dead, dead, dead)
        };
        scene.draw_rect(outer, theme.surface.fade(o), DOT * 0.5, 2.0, ring.fade(o));
        if self.selected {
            let inner = DOT * 0.5;
            let pad = (DOT - inner) * 0.5;
            scene.draw_rect(
                Rect::new(outer.x + pad, outer.y + pad, inner, inner),
                dot.fade(o),
                inner * 0.5,
                0.0,
                Color::TRANSPARENT,
            );
        }
        scene.text(
            Point::new(bounds.x + DOT + GAP, bounds.y),
            self.label.clone(),
            self.size,
            label.fade(o),
        );
    }

    fn on_click(&self) -> Option<Msg> {
        if !self.enabled {
            return None;
        }
        self.on_click.clone()
    }

    fn focusable(&self) -> bool {
        self.enabled
    }

    fn semantics(&self) -> Option<frus_core::Semantics> {
        // A radio group had no semantics at all before this: a reader was told nothing
        // about which option was chosen. Adding the disabled announcement without the
        // announcement it qualifies would have been announcing an absence.
        let semantics = frus_core::Semantics::new(frus_core::Role::RadioButton)
            .label(self.label.clone())
            .toggled(self.selected);
        Some(if self.enabled {
            semantics.clickable()
        } else {
            semantics.disabled(true)
        })
    }
}

/// A single-selection group of radio buttons.
pub struct RadioGroup<Msg> {
    selected: usize,
    size: f32,
    gap: f32,
    enabled: bool,
    on_select: Box<dyn Fn(usize) -> Msg>,
    /// The labels as given. The options are **derived** from these, so that a builder
    /// called after them — `enabled`, and any that follow — still reaches every option
    /// rather than only the ones added afterwards.
    labels: Vec<String>,
    options: Vec<Box<dyn Widget<Msg>>>,
}

impl<Msg: Clone + 'static> RadioGroup<Msg> {
    /// Creates a group: the selected index plus an `index -> message` closure.
    pub fn new(selected: usize, on_select: impl Fn(usize) -> Msg + 'static) -> Self {
        Self {
            selected,
            size: 18.0,
            gap: 8.0,
            enabled: true,
            on_select: Box::new(on_select),
            labels: Vec::new(),
            options: Vec::new(),
        }
    }

    /// Adds an option, in order.
    pub fn option(mut self, label: impl Into<String>) -> Self {
        self.labels.push(label.into());
        self.rebuild();
        self
    }

    /// Whether the group can be chosen from. Disabled it is **inert** — no message, out
    /// of the tab order, announced as unavailable — and it still shows which option is
    /// chosen.
    ///
    /// It disables the **whole** group; a single unavailable option among live ones is
    /// not expressible yet. See [`crate::disabled`] for the whole contract.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self.rebuild();
        self
    }

    /// Rebuilds the options from the labels, so that the order of the builders does not
    /// change what comes out.
    fn rebuild(&mut self) {
        self.options = self
            .labels
            .iter()
            .enumerate()
            .map(|(index, label)| {
                Box::new(RadioOption {
                    label: label.clone(),
                    selected: index == self.selected,
                    size: self.size,
                    enabled: self.enabled,
                    on_click: Some((self.on_select)(index)),
                }) as Box<dyn Widget<Msg>>
            })
            .collect();
    }
}

impl<Msg: Clone> Widget<Msg> for RadioGroup<Msg> {
    fn style(&self) -> Style {
        Style {
            flex_direction: FlexDirection::Column,
            gap: self.gap,
            ..Default::default()
        }
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &self.options
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
        Pick(usize),
    }

    fn group(enabled_first: bool) -> RadioGroup<Msg> {
        // The same group built in both orders. `enabled` before the options and `enabled`
        // after them must come out identical — the reason the options are derived from
        // the labels rather than frozen as each one is added.
        if enabled_first {
            RadioGroup::new(1, Msg::Pick)
                .enabled(false)
                .option("Daily")
                .option("Weekly")
        } else {
            RadioGroup::new(1, Msg::Pick)
                .option("Daily")
                .option("Weekly")
                .enabled(false)
        }
    }

    #[test]
    fn picking_an_option_reports_its_index() {
        let live = RadioGroup::new(0, Msg::Pick).option("Daily").option("Weekly");
        let options = Widget::children(&live);
        assert_eq!(options.len(), 2);
        assert_eq!(options[1].on_click(), Some(Msg::Pick(1)));
    }

    #[test]
    fn a_disabled_group_disables_every_option_whichever_order_it_was_built_in() {
        for first in [true, false] {
            let dead = group(first);
            let options = Widget::children(&dead);
            assert_eq!(options.len(), 2, "built in order {first}");
            for (i, option) in options.iter().enumerate() {
                assert_eq!(option.on_click(), None, "option {i} still answers");
                assert!(!option.focusable(), "option {i} is still in the tab order");
                let semantics = option.semantics().expect("still announced");
                assert!(semantics.disabled, "option {i} does not say it is disabled");
            }
            // And the chosen one is still legible to a reader who cannot change it.
            assert_eq!(
                options[1].semantics().unwrap().toggled,
                frus_core::Toggled::True
            );
        }
    }

    /// The trap the rebuild exists for: with the options frozen as they were added,
    /// `.enabled(false)` at the end of the chain would have reached none of them, and the
    /// group would have looked disabled to a reader of the call site and answered every
    /// tap.
    #[test]
    fn the_builder_order_does_not_change_what_comes_out() {
        let before = Widget::children(&group(true))
            .iter()
            .map(|o| o.focusable())
            .collect::<Vec<_>>();
        let after = Widget::children(&group(false))
            .iter()
            .map(|o| o.focusable())
            .collect::<Vec<_>>();
        assert_eq!(before, after);
        assert_eq!(after, vec![false, false]);
    }
}
