//! What the form produces: the account it creates, and the small types it is made of.
//!
//! Plain data — no widget, no framework. The form fills these in; the summary page shows
//! them.

use std::fmt;

/// A calendar date. The fields are ordered so that `<` is chronological order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Date {
    pub year: i32,
    /// 1 to 12.
    pub month: u32,
    /// 1 to 31.
    pub day: u32,
}

impl Date {
    pub const fn new(year: i32, month: u32, day: u32) -> Self {
        Self { year, month, day }
    }

    /// The number of whole years from `self` to `later` — an age, when `self` is a birth
    /// date and `later` is today.
    pub fn years_until(self, later: Date) -> i32 {
        let years = later.year - self.year;
        // Not yet had this year's birthday: one year fewer.
        if (later.month, later.day) < (self.month, self.day) {
            years - 1
        } else {
            years
        }
    }
}

/// `2001-04-09`: the format nobody misreads, whichever way round their dates usually go.
impl fmt::Display for Date {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:04}-{:02}-{:02}", self.year, self.month, self.day)
    }
}

/// The subscription the account starts on.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Plan {
    #[default]
    Free,
    Pro,
    Team,
}

impl Plan {
    /// In the order the form lists them; a plan's index here is its radio button's.
    pub const ALL: [Plan; 3] = [Plan::Free, Plan::Pro, Plan::Team];

    pub fn name(self) -> &'static str {
        match self {
            Plan::Free => "Free",
            Plan::Pro => "Pro",
            Plan::Team => "Team",
        }
    }

    /// What the radio button says: the name and the price. Short, because a radio label
    /// stays on one line.
    pub fn label(self) -> &'static str {
        match self {
            Plan::Free => "Free",
            Plan::Pro => "Pro · $8/month",
            Plan::Team => "Team · $20/month",
        }
    }

    pub fn index(self) -> usize {
        Plan::ALL.iter().position(|&p| p == self).unwrap_or(0)
    }

    pub fn from_index(index: usize) -> Plan {
        Plan::ALL.get(index).copied().unwrap_or_default()
    }
}

/// The countries the dropdown offers.
pub const COUNTRIES: [&str; 10] = [
    "Benin",
    "Brazil",
    "Canada",
    "France",
    "Germany",
    "India",
    "Japan",
    "Nigeria",
    "Togo",
    "United States",
];

/// The topics a person can follow, shown as filter chips.
pub const INTERESTS: [&str; 6] = ["Design", "Mobile", "Web", "Games", "AI", "Open source"];

/// An account, as a valid submission describes it.
///
/// There is no password here on purpose: once the account is created, nothing on the
/// client should keep it.
#[derive(Clone, Debug, PartialEq)]
pub struct Profile {
    pub full_name: String,
    pub email: String,
    pub phone: String,
    pub birth_date: Date,
    pub country: String,
    pub plan: Plan,
    pub interests: Vec<String>,
    pub newsletter: bool,
    pub bio: String,
}

impl Profile {
    /// A believable account: what "Fill with sample data" types into the form, and what the
    /// summary page shows when it is opened directly, with nothing submitted.
    pub fn sample() -> Self {
        Self {
            full_name: "Ada Lovelace".to_string(),
            email: "ada@example.com".to_string(),
            phone: "+44 20 7946 0958".to_string(),
            birth_date: Date::new(1995, 12, 10),
            country: "France".to_string(),
            plan: Plan::Pro,
            interests: vec!["Design".to_string(), "Open source".to_string()],
            newsletter: true,
            bio: "Writes programs for machines that do not exist yet.".to_string(),
        }
    }
}
