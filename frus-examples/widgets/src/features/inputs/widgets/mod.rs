//! The demos of this category, in the order the gallery lists them.

use crate::models::demo::Demo;

mod autocomplete;
mod autofill_group;
mod checkbox;
mod color_picker;
mod control_tiles;
mod date_picker;
mod date_time_picker;
mod date_time_range;
mod form;
mod radio;
mod range_slider;
mod rating;
mod search_anchor;
mod search_bar;
mod slider;
mod stepper;
mod switch;
mod text_editing_controller;
mod text_field;
mod time_picker;

pub(crate) fn demos() -> Vec<Demo> {
    vec![
        Demo::new(
            "text-field",
            "TextField",
            "A text input: labels, hints, errors, icons, passwords, several lines and filters.",
            include_str!("text_field.rs"),
            text_field::example,
        ),
        Demo::new(
            "text-editing-controller",
            "TextEditingController",
            "A field's text held outside the field, to read, fill and clear it from code.",
            include_str!("text_editing_controller.rs"),
            text_editing_controller::example,
        ),
        Demo::new(
            "form",
            "Form, Rule & ErrorSummary",
            "Validation as a plain function of the state, shown on the fields and in a summary.",
            include_str!("form.rs"),
            form::example,
        ),
        Demo::new(
            "autocomplete",
            "Autocomplete",
            "A text field that suggests completions in a floating list.",
            include_str!("autocomplete.rs"),
            autocomplete::example,
        ),
        Demo::new(
            "search-bar",
            "SearchBar",
            "The raised pill an application searches from.",
            include_str!("search_bar.rs"),
            search_bar::example,
        ),
        Demo::new(
            "search-anchor",
            "SearchAnchor",
            "A search bar that opens a search view with suggestions.",
            include_str!("search_anchor.rs"),
            search_anchor::example,
        ),
        Demo::new(
            "checkbox",
            "Checkbox",
            "A yes/no choice with an optional label, and a three-state version.",
            include_str!("checkbox.rs"),
            checkbox::example,
        ),
        Demo::new(
            "radio",
            "RadioGroup & Radio",
            "One choice out of a few, all visible at once.",
            include_str!("radio.rs"),
            radio::example,
        ),
        Demo::new(
            "switch",
            "Switch",
            "An on/off toggle that takes effect at once.",
            include_str!("switch.rs"),
            switch::example,
        ),
        Demo::new(
            "control-list-tiles",
            "CheckboxListTile, RadioListTile & SwitchListTile",
            "List rows whose whole width works one control, for settings screens.",
            include_str!("control_tiles.rs"),
            control_tiles::example,
        ),
        Demo::new(
            "slider",
            "Slider",
            "A value picked by dragging along a track, continuous or in steps.",
            include_str!("slider.rs"),
            slider::example,
        ),
        Demo::new(
            "range-slider",
            "RangeSlider",
            "Two thumbs on one track, for picking an interval.",
            include_str!("range_slider.rs"),
            range_slider::example,
        ),
        Demo::new(
            "rating",
            "Rating",
            "A score given by tapping stars.",
            include_str!("rating.rs"),
            rating::example,
        ),
        Demo::new(
            "stepper",
            "Stepper",
            "A whole number with − and + buttons on either side.",
            include_str!("stepper.rs"),
            stepper::example,
        ),
        Demo::new(
            "date-picker",
            "DatePicker",
            "A month calendar: one day, allowed days only, or a range.",
            include_str!("date_picker.rs"),
            date_picker::example,
        ),
        Demo::new(
            "time-picker",
            "TimePicker & TimeRange",
            "A time of day from an hour grid and a minute grid; or a start and an end.",
            include_str!("time_picker.rs"),
            time_picker::example,
        ),
        Demo::new(
            "date-time-picker",
            "DateTimePicker",
            "A calendar above a time picker, for one moment.",
            include_str!("date_time_picker.rs"),
            date_time_picker::example,
        ),
        Demo::new(
            "date-time-range",
            "DateTimeRange",
            "A date range over two months and a start and end time, for booking a slot.",
            include_str!("date_time_range.rs"),
            date_time_range::example,
        ),
        Demo::new(
            "color-picker",
            "ColorPicker",
            "A grid of colour swatches to pick from.",
            include_str!("color_picker.rs"),
            color_picker::example,
        ),
        Demo::new(
            "autofill-group",
            "AutofillGroup & AutofillHint",
            "Tell the platform what each field holds so it can be filled in.",
            include_str!("autofill_group.rs"),
            autofill_group::example,
        ),
    ]
}
