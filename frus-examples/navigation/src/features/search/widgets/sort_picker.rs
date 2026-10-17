use std::rc::Rc;

use frus::{BuildContext, Callback, SegmentedButton, StatelessWidget, Widget};

use crate::features::search::states::searchstate::Sort;

/// The three orders, as a segmented button. The parent says what picking one does.
pub struct SortPicker {
    selected: Sort,
    on_pick: Rc<dyn Fn(Sort) -> Callback>,
}

impl SortPicker {
    pub fn new(selected: Sort, on_pick: impl Fn(Sort) -> Callback + 'static) -> Self {
        Self {
            selected,
            on_pick: Rc::new(on_pick),
        }
    }
}

impl StatelessWidget for SortPicker {
    fn build(&self, _cx: &BuildContext) -> Box<dyn Widget> {
        let on_pick = self.on_pick.clone();
        let picker = SegmentedButton::new(self.selected.index(), move |index: usize| {
            on_pick(Sort::ALL[index])
        });
        Box::new(
            Sort::ALL
                .iter()
                .fold(picker, |picker, sort| picker.segment(sort.label())),
        )
    }
}
