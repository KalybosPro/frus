//! What the post page keeps between rebuilds.

pub(crate) struct PostState {
    /// Which attempt this is. The page's three requests are keyed on it, so Retry —
    /// which bumps it — sends all of them again.
    pub attempt: u64,
}

impl PostState {
    pub(crate) fn new() -> Self {
        Self { attempt: 0 }
    }

    pub(crate) fn retry(&mut self) {
        self.attempt += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retry_is_a_new_attempt() {
        let mut state = PostState::new();
        state.retry();
        state.retry();
        assert_eq!(state.attempt, 2);
    }
}
