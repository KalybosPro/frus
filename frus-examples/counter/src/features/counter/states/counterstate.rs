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

    /// What survives a reload under `frus run` (`r`): the count and the step, as text.
    pub(crate) fn to_saved(&self) -> String {
        format!("{} {}", self.count, self.step)
    }

    /// Takes back what [`CounterState::to_saved`] wrote. Text from an older version that no
    /// longer reads is ignored: the counter starts afresh.
    pub(crate) fn take_saved(&mut self, saved: &str) {
        let mut numbers = saved.split_whitespace().map(str::parse::<i64>);
        if let (Some(Ok(count)), Some(Ok(step))) = (numbers.next(), numbers.next()) {
            self.count = count;
            self.step = step;
        }
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
    fn the_count_survives_a_reload() {
        let mut before = CounterState::new(1);
        before.set_step(5);
        before.increment();
        let mut after = CounterState::new(1);
        after.take_saved(&before.to_saved());
        assert_eq!((after.count, after.step), (5, 5));

        let mut fresh = CounterState::new(1);
        fresh.take_saved("not a counter");
        assert_eq!((fresh.count, fresh.step), (0, 1));
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
