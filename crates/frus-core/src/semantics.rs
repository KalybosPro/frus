//! **Accessibility semantics**: the per-widget annotation (role, label, value,
//! state) that the framework exposes to assistive technology.
//!
//! A frus-native, dependency-free type that **maps** onto `accesskit` at the
//! platform edge (the shell builds an AccessKit tree out of these nodes). This
//! follows the §14 advice: *bake the label into the widgets now, wire AccessKit up
//! afterwards.*

/// An element's semantic **role** (a subset aligned on AccessKit/ARIA roles: what a
/// screen reader announces).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Role {
    /// An element with no role of its own (a layout container).
    #[default]
    None,
    /// Static text (label, paragraph).
    Label,
    /// A heading or title.
    Heading,
    /// An actionable button.
    Button,
    /// A link.
    Link,
    /// A checkbox (`checked` state).
    CheckBox,
    /// A switch (`checked` state).
    Switch,
    /// A radio button (`checked` state).
    RadioButton,
    /// A continuous-value slider (`value` / `min` / `max`).
    Slider,
    /// A text input field (`value` = its contents).
    TextInput,
    /// An image or icon described by its `label`.
    Image,
    /// A tab.
    Tab,
    /// A list item.
    ListItem,
    /// A progress bar (`value`).
    ProgressBar,
}

/// The checked state of a toggleable control (checkbox, switch, radio).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Toggled {
    /// Not applicable — the control does not toggle.
    #[default]
    None,
    /// Unchecked.
    False,
    /// Checked.
    True,
}

/// A widget's **resolved** semantic annotation, for accessibility.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct Semantics {
    /// The announced role.
    pub role: Role,
    /// Accessible name (what the screen reader reads out).
    pub label: Option<String>,
    /// Textual value (a field's contents, a slider's position, and so on).
    pub value: Option<String>,
    /// Checked state (checkboxes, switches, radios).
    pub toggled: Toggled,
    /// Actionable by click — the screen reader offers "activate".
    pub clickable: bool,
    /// Disabled (greyed out, non-interactive).
    pub disabled: bool,
    /// Numeric bounds `(min, value, max)` for sliders and progress bars.
    pub range: Option<(f32, f32, f32)>,
}

impl Semantics {
    /// An annotation carrying the given role and nothing else.
    pub fn new(role: Role) -> Self {
        Self {
            role,
            ..Default::default()
        }
    }

    /// Sets the accessible name.
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Sets the textual value.
    pub fn value(mut self, value: impl Into<String>) -> Self {
        self.value = Some(value.into());
        self
    }

    /// Marks the checked state.
    pub fn toggled(mut self, on: bool) -> Self {
        self.toggled = if on { Toggled::True } else { Toggled::False };
        self
    }

    /// Marks the element as actionable.
    pub fn clickable(mut self) -> Self {
        self.clickable = true;
        self
    }

    /// Marks the element as disabled.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Sets the numeric bounds `(min, value, max)`.
    pub fn range(mut self, min: f32, value: f32, max: f32) -> Self {
        self.range = Some((min, value, max));
        self
    }

    /// `true` when this node carries something useful to assistive technology — a
    /// non-null role or a label. Empty containers are left out of the tree.
    pub fn is_meaningful(&self) -> bool {
        self.role != Role::None || self.label.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builders_compose() {
        let s = Semantics::new(Role::CheckBox)
            .label("Notifications")
            .toggled(true);
        assert_eq!(s.role, Role::CheckBox);
        assert_eq!(s.label.as_deref(), Some("Notifications"));
        assert_eq!(s.toggled, Toggled::True);
        assert!(s.is_meaningful());
    }

    #[test]
    fn empty_is_not_meaningful() {
        assert!(!Semantics::default().is_meaningful());
        // A bare label is enough to be exposed.
        assert!(Semantics::default().label("x").is_meaningful());
    }
}
