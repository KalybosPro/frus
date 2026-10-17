//! What the post page keeps between rebuilds.

pub struct PostState {
    /// Which attempt this is. The page's three requests are keyed on it, so Retry —
    /// which bumps it — sends all of them again.
    pub attempt: u64,
}

impl PostState {
    pub fn new() -> Self {
        Self { attempt: 0 }
    }

    pub fn retry(&mut self) {
        self.attempt += 1;
    }
}

impl Default for PostState {
    fn default() -> Self {
        Self::new()
    }
}
