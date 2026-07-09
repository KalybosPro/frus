//! `frus-widgets` — l'arbre de widgets déclaratif de frus.
//!
//! On décrit l'interface avec des widgets composables ([`Container`], [`Flex`])
//! génériques sur un type de message `Msg` (modèle à messages, façon Elm/iced).
//! [`build_ui`] les traduit en mise en page (via `frus-layout`) puis en une
//! [`Ui`] : la [`Scene`] à dessiner + une carte de hit-test pour router les clics.
//!
//! L'état visuel de survol/pression et le clavier viendront avec la
//! reconciliation d'arbre (jalon ultérieur).

mod container;
mod flex;
mod interaction;
mod scroll;
mod text;
mod textinput;
mod ui;
mod widget;

pub use container::Container;
pub use flex::Flex;
pub use interaction::{InputState, Interaction, Key, Status, WidgetId};
pub use scroll::Scroll;
pub use text::Text;
pub use textinput::TextInput;
pub use ui::{build_ui, dispatch_key, ScrollState, Ui};
pub use widget::Widget;

// Ré-exports de commodité pour les appelants.
pub use frus_core::{Color, Insets, Point, Rect, Scene, Size};
pub use frus_layout::{Align, Justify};
