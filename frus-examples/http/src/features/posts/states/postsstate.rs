//! What the posts page keeps between rebuilds: not the posts themselves — the request
//! hook keeps those — but what decides *when* to ask for them again.

pub struct PostsState {
    /// Which load this is. The request hook runs once per value, so changing it is how
    /// the page asks for the list again.
    pub generation: u64,
    /// Whether the load in flight was asked for by pulling the list down. The pull's own
    /// spinner then says it is working; a reload from the app bar shows a bar instead.
    pub pulled: bool,
}

impl PostsState {
    pub fn new() -> Self {
        Self {
            generation: 0,
            pulled: false,
        }
    }

    /// The Retry button and the app bar's refresh action.
    pub fn reload(&mut self) {
        self.generation += 1;
        self.pulled = false;
    }

    /// A pull-to-refresh.
    pub fn pull_to_refresh(&mut self) {
        self.generation += 1;
        self.pulled = true;
    }
}

impl Default for PostsState {
    fn default() -> Self {
        Self::new()
    }
}
