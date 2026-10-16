//! What the counter page keeps between rebuilds.
//!
//! A plain struct with plain methods: nothing here knows about widgets, so the rules are
//! tested below with no window and no GPU.

pub(crate) struct CounterState {
    pub count: i64,
    /// How much one press adds or removes. Starts from the widget's configuration.
    pub step: i64,
    /// While it is on, a one-second timer increments the count.
    pub auto: bool,
}

impl CounterState {
    pub(crate) fn new(step: i64) -> Self {
        Self {
            count: 0,
            step,
            auto: false,
        }
    }

    pub(crate) fn increment(&mut self) {
        self.count += self.step;
    }

    pub(crate) fn decrement(&mut self) {
        self.count -= self.step;
    }

    pub(crate) fn reset(&mut self) {
        self.count = 0;
        self.auto = false;
    }

    pub(crate) fn toggle_auto(&mut self) {
        self.auto = !self.auto;
    }

    pub(crate) fn set_step(&mut self, step: i64) {
        self.step = step;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counting_follows_the_step() {
        let mut state = CounterState::new(1);
        state.increment();
        state.increment();
        state.decrement();
        assert_eq!(state.count, 1);

        state.set_step(10);
        state.increment();
        assert_eq!(state.count, 11);
    }

    #[test]
    fn reset_also_stops_the_timer() {
        let mut state = CounterState::new(1);
        state.increment();
        state.toggle_auto();
        state.reset();
        assert_eq!(state.count, 0);
        assert!(!state.auto);
    }
}
