//! What the posts page keeps between rebuilds: not the posts themselves — the request
//! hook keeps those — but what decides *when* to ask for them again.

pub(crate) struct PostsState {
    /// Which load this is. The request hook runs once per value, so changing it is how
    /// the page asks for the list again.
    pub generation: u64,
    /// Whether the load in flight was asked for by pulling the list down. The pull's own
    /// spinner then says it is working; a reload from the app bar shows a bar instead.
    pub pulled: bool,
}

impl PostsState {
    pub(crate) fn new() -> Self {
        Self {
            generation: 0,
            pulled: false,
        }
    }

    /// The Retry button and the app bar's refresh action.
    pub(crate) fn reload(&mut self) {
        self.generation += 1;
        self.pulled = false;
    }

    /// A pull-to-refresh.
    pub(crate) fn pull_to_refresh(&mut self) {
        self.generation += 1;
        self.pulled = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_reload_is_a_new_generation() {
        let mut state = PostsState::new();
        state.reload();
        state.pull_to_refresh();
        assert_eq!(state.generation, 2);
    }

    #[test]
    fn the_state_remembers_who_asked() {
        let mut state = PostsState::new();
        state.pull_to_refresh();
        assert!(state.pulled);
        state.reload();
        assert!(!state.pulled);
    }
}
