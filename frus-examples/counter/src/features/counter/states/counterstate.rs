//! What the counter page keeps between rebuilds.
//!
//! A plain struct with plain methods: nothing here knows about widgets, so the rules are
//! tested in `tests/counterstate_test.rs` with no window and no GPU.

pub struct CounterState {
    pub count: i64,
    /// How much one press adds or removes. Starts from the widget's configuration.
    pub step: i64,
    /// While it is on, a one-second timer increments the count.
    pub auto: bool,
}

impl CounterState {
    pub fn new(step: i64) -> Self {
        Self {
            count: 0,
            step,
            auto: false,
        }
    }

    pub fn increment(&mut self) {
        self.count += self.step;
    }

    pub fn decrement(&mut self) {
        self.count -= self.step;
    }

    pub fn reset(&mut self) {
        self.count = 0;
        self.auto = false;
    }

    pub fn toggle_auto(&mut self) {
        self.auto = !self.auto;
    }

    pub fn set_step(&mut self, step: i64) {
        self.step = step;
    }

    /// What survives a reload under `frus run` (`r`): the count and the step, as text.
    pub fn to_saved(&self) -> String {
        format!("{} {}", self.count, self.step)
    }

    /// Takes back what [`CounterState::to_saved`] wrote. Text from an older version that no
    /// longer reads is ignored: the counter starts afresh.
    pub fn take_saved(&mut self, saved: &str) {
        let mut numbers = saved.split_whitespace().map(str::parse::<i64>);
        if let (Some(Ok(count)), Some(Ok(step))) = (numbers.next(), numbers.next()) {
            self.count = count;
            self.step = step;
        }
    }
}
