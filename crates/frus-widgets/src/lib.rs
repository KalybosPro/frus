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

mod container;
mod flex;
mod interaction;
mod runtime;
mod scroll;
mod text;
mod textinput;
mod ui;
mod widget;

pub use container::Container;
pub use flex::Flex;
pub use interaction::{InputState, Interaction, Key, Status, WidgetId};
pub use runtime::{Edit, Runtime, ScrollState};
pub use scroll::Scroll;
pub use text::Text;
pub use textinput::TextInput;
pub use ui::{build_ui, find_widget, Ui};
pub use widget::Widget;

// Ré-exports de commodité pour les appelants.
pub use frus_core::{Color, Insets, Point, Rect, Scene, Size};
pub use frus_layout::{Align, Justify};
