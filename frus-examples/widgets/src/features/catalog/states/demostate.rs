/// What a demo page remembers: which tab is showing.
pub struct DemoState {
    pub tab: usize,
}

impl DemoState {
    pub fn select_tab(&mut self, tab: usize) {
        self.tab = tab;
    }
}
