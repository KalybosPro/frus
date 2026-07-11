//! `frus-widgets` — l'arbre de widgets déclaratif de frus.
//!
//! On décrit l'interface avec des widgets composables ([`Container`], [`Flex`],
//! [`Text`], [`TextInput`], [`Scroll`]) génériques sur un type de message `Msg`
//! (modèle à messages, façon Elm/iced). [`build_ui`] les traduit en mise en page
//! (via `frus-layout`) puis en une [`Ui`] : la [`Scene`] à dessiner + des cartes
//! de hit-test (clic, focus, scroll).
//!
//! L'état retenu entre frames (survol/focus, offsets de scroll, curseur/sélection)
//! vit dans un [`Runtime`], clé par identité de widget.

mod alert;
mod autocomplete;
mod avatar;
mod badge;
mod breadcrumb;
mod button;
mod card;
mod carousel;
mod checkbox;
mod chip;
mod collapsible;
mod colorpicker;
mod container;
mod datepicker;
mod divider;
mod dropdown;
mod dsl;
mod flex;
mod grid;
mod interaction;
mod kbd;
mod keyed;
mod list;
mod menu;
mod popover;
mod navbar;
mod navigator;
mod pagination;
mod portal;
mod progressbar;
mod radio;
mod rating;
mod runtime;
mod scroll;
mod segmented;
mod skeleton;
mod slider;
mod spinner;
mod stack;
mod stepper;
mod switch;
mod table;
mod tabs;
mod text;
mod textinput;
mod theme;
mod timeline;
mod toast;
mod tree;
mod ui;
mod widget;

pub use alert::{Alert, AlertKind};
pub use autocomplete::Autocomplete;
pub use avatar::Avatar;
pub use badge::Badge;
pub use breadcrumb::Breadcrumb;
pub use button::{Button, Variant};
pub use card::Card;
pub use carousel::Carousel;
pub use checkbox::Checkbox;
pub use chip::Chip;
pub use collapsible::Collapsible;
pub use colorpicker::ColorPicker;
pub use container::Container;
pub use datepicker::DatePicker;
pub use divider::Divider;
pub use dropdown::Dropdown;
pub use dsl::{button, keyed, spacer, text};
pub use flex::Flex;
pub use grid::Grid;
pub use interaction::{InputState, Interaction, Key, Status, WidgetId};
pub use kbd::Kbd;
pub use keyed::Keyed;
pub use list::{List, VirtualList};
pub use menu::Menu;
pub use popover::Popover;
pub use navbar::NavBar;
pub use navigator::Navigator;
pub use pagination::Pagination;
pub use portal::{Placement, Portal};
pub use progressbar::ProgressBar;
pub use radio::RadioGroup;
pub use rating::Rating;
pub use runtime::{spring_step, Anim, Edit, Runtime, ScrollState};
pub use scroll::{Axis, Scroll};
pub use segmented::SegmentedControl;
pub use skeleton::Skeleton;
pub use slider::Slider;
pub use spinner::Spinner;
pub use stack::Stack;
pub use stepper::Stepper;
pub use switch::Switch;
pub use table::Table;
pub use tabs::Tabs;
pub use text::Text;
pub use textinput::TextInput;
pub use theme::Theme;
pub use timeline::Timeline;
pub use toast::{Toast, ToastKind};
pub use tree::Tree;
pub use ui::{build_ui, collect_ids, find_widget, Scrollbar, Ui};
pub use widget::Widget;

// Ré-exports de commodité pour les appelants.
pub use frus_core::{Color, Insets, Point, Rect, Scene, Size};
pub use frus_layout::{Align, Justify};
