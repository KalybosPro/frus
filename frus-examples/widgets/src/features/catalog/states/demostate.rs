/// What a demo page remembers: which tab is showing.
pub(crate) struct DemoState {
    pub tab: usize,
}

impl DemoState {
    pub(crate) fn select_tab(&mut self, tab: usize) {
        self.tab = tab;
    }
}
