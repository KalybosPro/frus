//! The demos of this category, in the order the gallery lists them.

use crate::models::demo::Demo;

pub mod future_builder;
pub mod hooks;
pub mod host_tasks;
pub mod keyed;
pub mod localizations;
pub mod media_query;
pub mod notifiers;
pub mod responsive;
pub mod stateful_widget;
pub mod stateless_widget;
pub mod stream_builder;
pub mod theme;
pub mod themed;
pub mod use_effect;
pub mod use_interval;

pub fn demos() -> Vec<Demo> {
    vec![
        Demo::new(
            "stateless-widget",
            "StatelessWidget & component",
            "A piece of interface built from its configuration, as a struct or a closure.",
            include_str!("stateless_widget.rs"),
            stateless_widget::example,
        ),
        Demo::new(
            "stateful-widget",
            "StatefulWidget & State",
            "State kept between rebuilds, and its lifecycle: init, update, dispose.",
            include_str!("stateful_widget.rs"),
            stateful_widget::example,
        ),
        Demo::new(
            "hooks",
            "use_state, use_ref & use_memo",
            "State kept by a function component, with no struct to declare.",
            include_str!("hooks.rs"),
            hooks::example,
        ),
        Demo::new(
            "use-effect",
            "use_effect",
            "Work done after a build, rerun when its dependencies change, with a cleanup.",
            include_str!("use_effect.rs"),
            use_effect::example,
        ),
        Demo::new(
            "use-interval",
            "use_interval",
            "A timer owned by a component, on for as long as builds ask for it.",
            include_str!("use_interval.rs"),
            use_interval::example,
        ),
        Demo::new(
            "value-notifier",
            "ValueNotifier, ChangeNotifier & their builders",
            "State outside the widgets that tells whoever cares when it changed.",
            include_str!("notifiers.rs"),
            notifiers::example,
        ),
        Demo::new(
            "future-builder",
            "use_future & FutureBuilder",
            "Work that finishes later: waiting, then the result.",
            include_str!("future_builder.rs"),
            future_builder::example,
        ),
        Demo::new(
            "stream-builder",
            "use_stream & StreamBuilder",
            "Work that gives several values as it goes, then finishes.",
            include_str!("stream_builder.rs"),
            stream_builder::example,
        ),
        Demo::new(
            "host-spawn",
            "host::spawn & host::after",
            "Slow work on another thread, and something to do after a delay.",
            include_str!("host_tasks.rs"),
            host_tasks::example,
        ),
        Demo::new(
            "keyed",
            "keyed & Keyed",
            "A stable identity, so a widget's state follows it when siblings move.",
            include_str!("keyed.rs"),
            keyed::example,
        ),
        Demo::new(
            "theme",
            "Theme",
            "Reading the theme's colours and type, and switching light and dark.",
            include_str!("theme.rs"),
            theme::example,
        ),
        Demo::new(
            "themed",
            "Themed & ThemeBuilder",
            "A different theme, or part of one, for one part of the screen.",
            include_str!("themed.rs"),
            themed::example,
        ),
        Demo::new(
            "media-query",
            "MediaQuery & MediaScope",
            "The screen's size, insets, text scale, brightness and size class.",
            include_str!("media_query.rs"),
            media_query::example,
        ),
        Demo::new(
            "responsive",
            "Responsive, LayoutBuilder & OrientationBuilder",
            "One interface, several layouts, by window or by box.",
            include_str!("responsive.rs"),
            responsive::example,
        ),
        Demo::new(
            "localizations",
            "Locale & Localizations",
            "The interface's language, and the words the framework says in it.",
            include_str!("localizations.rs"),
            localizations::example,
        ),
    ]
}
