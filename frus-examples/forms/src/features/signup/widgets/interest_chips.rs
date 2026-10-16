use std::rc::Rc;

use frus::{BuildContext, Callback, Chip, StatelessWidget, Widget, Wrap};

use crate::models::profile::INTERESTS;

/// The interests as **filter chips**: each one toggles on its own, and the row wraps onto as
/// many lines as the width needs.
///
/// The chips report a press, not a value, so the page hands in a function from a chip's
/// index to the callback for it — `cx.handler(..)` makes exactly that.
pub(crate) struct InterestChips {
    selected: Vec<usize>,
    on_toggle: Rc<dyn Fn(usize) -> Callback>,
}

impl InterestChips {
    pub(crate) fn new(
        selected: Vec<usize>,
        on_toggle: impl Fn(usize) -> Callback + 'static,
    ) -> Self {
        Self {
            selected,
            on_toggle: Rc::new(on_toggle),
        }
    }
}

impl StatelessWidget for InterestChips {
    fn build(&self, _cx: &BuildContext) -> Box<dyn Widget> {
        let chips =
            INTERESTS
                .iter()
                .enumerate()
                .fold(Wrap::new().gap(8.0), |row, (index, &interest)| {
                    row.child(
                        Chip::new(interest)
                            .selected(self.selected.contains(&index))
                            // The tick is what tells a filter chip from a plain tag.
                            .show_checkmark(true)
                            .on_press((self.on_toggle)(index)),
                    )
                });
        Box::new(chips)
    }
}
