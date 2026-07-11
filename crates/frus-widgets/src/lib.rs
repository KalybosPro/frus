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

mod button;
mod card;
mod checkbox;
mod container;
mod dropdown;
mod dsl;
mod flex;
mod interaction;
mod keyed;
mod list;
mod navbar;
mod navigator;
mod portal;
mod radio;
mod runtime;
mod scroll;
mod slider;
mod switch;
mod text;
mod textinput;
mod theme;
mod ui;
mod widget;

pub use button::{Button, Variant};
pub use card::Card;
pub use checkbox::Checkbox;
pub use container::Container;
pub use dropdown::Dropdown;
pub use dsl::{button, keyed, spacer, text};
pub use flex::Flex;
pub use interaction::{InputState, Interaction, Key, Status, WidgetId};
pub use keyed::Keyed;
pub use list::{List, VirtualList};
pub use navbar::NavBar;
pub use navigator::Navigator;
pub use portal::{Placement, Portal};
pub use radio::RadioGroup;
pub use runtime::{spring_step, Anim, Edit, Runtime, ScrollState};
pub use scroll::{Axis, Scroll};
pub use slider::Slider;
pub use switch::Switch;
pub use text::Text;
pub use textinput::TextInput;
pub use theme::Theme;
pub use ui::{build_ui, collect_ids, find_widget, Scrollbar, Ui};
pub use widget::Widget;

// Ré-exports de commodité pour les appelants.
pub use frus_core::{Color, Insets, Point, Rect, Scene, Size};
pub use frus_layout::{Align, Justify};
