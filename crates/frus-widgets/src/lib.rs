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
mod animated;
mod appbar;
mod aspectratio;
mod autocomplete;
mod avatar;
mod badge;
mod bottomsheet;
mod breadcrumb;
mod button;
mod card;
mod carousel;
mod chart;
mod checkbox;
mod chip;
mod clip;
mod collapsible;
mod colorpicker;
mod container;
mod datatable;
mod datepicker;
mod datetimepicker;
mod datetimerange;
mod custompaint;
mod divider;
mod drawer;
mod dropdown;
mod dsl;
mod fittedbox;
mod flex;
/// Validation de formulaire (pure, côté application) : [`form::Rule`] et [`form::Form`].
pub mod form;
pub use form::ErrorSummary;
mod fractional;
mod grid;
mod icon;
mod icons;
mod image;
mod inspector;
mod interaction;
mod interactive;
mod kbd;
mod keyed;
mod layoutbuilder;
mod list;
mod menu;
mod popover;
mod navbar;
mod navigator;
mod navrail;
mod navscaffold;
mod pagination;
mod portal;
mod paintcache;
mod progressbar;
mod radio;
mod rating;
mod relayout;
mod responsive;
mod richtext;
mod rotatedbox;
mod runtime;
mod scaffold;
mod scroll;
mod segmented;
mod skeleton;
mod reorder;
mod slider;
mod spinner;
mod stack;
mod stepper;
mod steps;
mod switch;
mod table;
mod tabs;
mod text;
mod textinput;
mod theme;
mod timeline;
mod timepicker;
mod toast;
mod toasthost;
mod transform;
mod tree;
mod twopane;
mod ui;
mod widget;

pub use alert::{Alert, AlertKind};
pub use appbar::AppBar;
pub use aspectratio::AspectRatio;
pub use autocomplete::Autocomplete;
pub use avatar::Avatar;
pub use badge::Badge;
pub use bottomsheet::BottomSheet;
pub use breadcrumb::Breadcrumb;
pub use button::{Button, Variant};
pub use card::Card;
pub use carousel::Carousel;
pub use chart::{BarChart, LineChart};
pub use checkbox::Checkbox;
pub use chip::Chip;
pub use clip::{ClipOval, ClipPath, ClipRRect};
pub use collapsible::Collapsible;
pub use colorpicker::ColorPicker;
pub use animated::{AnimatedContainer, AnimatedOpacity, Opacity};
pub use container::Container;
pub use custompaint::CustomPaint;
pub use datatable::{compare_cells, page_count, page_range_label, page_rows, sort_rows, DataTable};
pub use datepicker::DatePicker;
pub use datetimepicker::DateTimePicker;
pub use datetimerange::DateTimeRange;
pub use divider::Divider;
pub use drawer::{Drawer, DRAWER_WIDTH};
pub use dropdown::Dropdown;
pub use dsl::{button, keyed, spacer, text};
pub use fittedbox::FittedBox;
pub use flex::{Flex, Wrap};
pub use fractional::FractionallySizedBox;
pub use grid::Grid;
pub use icon::Icon;
pub use icons::IconName;
pub use image::Image;
pub use interaction::{Cursor, InputState, Interaction, Key, KeyResponse, Status, WidgetId};
pub use interactive::{InteractiveView, InteractiveViewer};
pub use kbd::Kbd;
pub use keyed::Keyed;
pub use layoutbuilder::LayoutBuilder;
pub use list::{List, VirtualList};
pub use menu::Menu;
pub use popover::Popover;
pub use navbar::NavBar;
pub use navigator::Navigator;
pub use navrail::{BottomBar, NavRail};
pub use navscaffold::NavScaffold;
pub use pagination::Pagination;
pub use portal::{Placement, Portal};
pub use progressbar::ProgressBar;
pub use radio::RadioGroup;
pub use rating::Rating;
pub use paintcache::PaintCache;
pub use relayout::LayoutCache;
pub use responsive::{responsive, Responsive};
pub use richtext::RichText;
pub use rotatedbox::RotatedBox;
pub use runtime::{spring_ease, spring_step, Anim, Edit, Runtime, ScrollState};
pub use scaffold::{fab_button, Scaffold};
pub use scroll::{Axis, Scroll};
pub use segmented::SegmentedControl;
pub use skeleton::Skeleton;
pub use reorder::reflow_reorder_columns;
pub use slider::{RangeSlider, Slider};
pub use spinner::Spinner;
pub use stack::Stack;
pub use stepper::Stepper;
pub use steps::Steps;
pub use switch::Switch;
pub use table::Table;
pub use tabs::Tabs;
pub use text::Text;
pub use textinput::TextInput;
pub use theme::{ColorScheme, TextTheme, Theme};
pub use timeline::Timeline;
pub use timepicker::{Endpoint, TimeField, TimePicker, TimeRange};
pub use toast::{SnackbarQueue, Toast, ToastKind};
pub use toasthost::{ToastHost, ToastPosition};
pub use transform::Transform;
pub use tree::Tree;
pub use twopane::TwoPane;
pub use inspector::{dump_tree, node_at, paint_overlay as paint_inspector_overlay, InspectorNode};
pub use ui::{
    build_ui, build_ui_inspected, collect_ids, find_by_key, find_path, find_widget, FocusDirection,
    Scrollbar, Ui,
};
pub use widget::Widget;

// Ré-exports de commodité pour les appelants.
pub use frus_core::{
    Affine, Alignment, AlignmentDirectional, AlignmentGeometry, Border, BorderRadius, BoxDecoration,
    BoxFit, BoxShadow, ClipShape, Color, FontWeight, ImageData, ImageHandle, Insets, InsetsDirectional,
    LinearGradient, Orientation, Path, PathVerb, Point, Primitive, Rect, Role, Scene, Semantics,
    SizeClass, Size, TextDecoration, TextDirection, TextSpan, TextStyle, Toggled, WindowInsets,
};
/// Couche d'animation partagée (physique, courbes, pilote) — voir
/// [`frus_core::animation`]. Ré-exportée ici pour que les applications l'atteignent
/// via `frus_widgets` sans dépendre directement de `frus-core`.
pub use frus_core::{
    AnimationController, ClampedSimulation, Curve, FrictionSimulation, Lerp, Simulation,
    SpringDescription, SpringSimulation, Tolerance, Tween,
};
// `frus_core::Status` (avancement d'animation) est renommé pour ne pas masquer le
// `Status` d'interaction (état de peinture : survol/pression/focus…).
pub use frus_core::Status as AnimationStatus;
pub use frus_layout::{Align, Justify};
