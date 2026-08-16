//! `frus-widgets` — frus's declarative widget tree.
//!
//! An interface is described with composable widgets ([`Container`], [`Flex`],
//! [`Text`], [`TextInput`], [`Scroll`]) generic over a message type `Msg`, in the
//! Elm/iced message model. [`build_ui`] translates them into a layout, through
//! `frus-layout`, and then into a [`Ui`]: the [`Scene`] to draw, plus the hit-test maps
//! for clicking, focus and scrolling.
//!
//! The state retained between frames — hover and focus, scroll offsets, caret and
//! selection — lives in a [`Runtime`], keyed by widget identity.

mod alert;
mod animated;
mod appbar;
mod aspectratio;
mod autocomplete;
mod avatar;
mod badge;
mod barrier;
mod bottomappbar;
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
mod constraints;
mod container;
mod custompaint;
mod datatable;
mod datepicker;
mod datetimepicker;
mod datetimerange;
/// Swipe-to-dismiss: the [`dismiss::Dismissible`] widget and its retained state.
pub mod dismiss;
mod divider;
mod dragdrop;
mod drawer;
mod dropdown;
mod dsl;
mod fittedbox;
mod flex;
/// Form validation, pure and application-side: [`form::Rule`] and [`form::Form`].
pub mod form;
pub use form::ErrorSummary;
mod fractional;
mod grid;
mod hero;
mod icon;
mod icons;
mod image;
mod ink;
mod inspector;
mod interaction;
mod interactive;
mod kanban;
mod kbd;
mod keyed;
mod layoutbuilder;
mod list;
mod media;
mod menu;
mod navbar;
mod navigator;
mod navrail;
mod navscaffold;
mod overscroll;
mod pageview;
mod pagination;
mod paintcache;
mod physics;
mod popover;
mod portal;
mod progressbar;
mod radio;
mod rating;
/// Pull-to-refresh: the [`refresh::Refresh`] widget and its retained pull.
pub mod refresh;
mod relayout;
mod reorder;
mod responsive;
mod richtext;
mod rotatedbox;
mod runtime;
mod safearea;
mod scaffold;
mod scroll;
mod segmented;
mod skeleton;
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
mod themed;
mod timeline;
mod timepicker;
mod toast;
mod toasthost;
mod transform;
pub(crate) mod transparent;
mod tree;
mod twopane;
mod ui;
mod widget;
mod widgettheme;

pub use alert::{Alert, AlertKind};
pub use animated::{AnimatedContainer, AnimatedOpacity, Opacity};
pub use appbar::{platform_centers_title, AppBar};
pub use aspectratio::AspectRatio;
pub use autocomplete::Autocomplete;
pub use avatar::Avatar;
pub use badge::Badge;
pub use barrier::{AbsorbPointer, Barrier, ExcludeSemantics, IgnorePointer, Offstage, Visibility};
pub use bottomappbar::{bar_spacer, notched_outline, BottomAppBar};
pub use bottomsheet::BottomSheet;
pub use breadcrumb::Breadcrumb;
pub use button::{
    Button, Variant, BUTTON_BORDER_WIDTH, BUTTON_ELEVATION, BUTTON_HEIGHT, BUTTON_MIN_WIDTH,
    BUTTON_PADDING, BUTTON_TEXT_PADDING,
};
pub use card::{Card, CardVariant, CARD_ELEVATION, CARD_MARGIN};
pub use carousel::Carousel;
pub use chart::{BarChart, LineChart};
pub use checkbox::Checkbox;
pub use chip::{
    Chip, CHIP_BORDER_WIDTH, CHIP_HEIGHT, CHIP_ICON_SIZE, CHIP_LABEL_PADDING, CHIP_PADDING,
    CHIP_RADIUS,
};
pub use clip::{ClipOval, ClipPath, ClipRRect};
pub use collapsible::Collapsible;
pub use colorpicker::ColorPicker;
pub use constraints::{
    ConstrainedBox, Intrinsic, IntrinsicAxis, IntrinsicHeight, IntrinsicWidth, Overflow,
    OverflowBox, SizedBox,
};
pub use container::Container;
pub use custompaint::CustomPaint;
pub use datatable::{
    compare_cells, page_count, page_range_label, page_rows, row_matches, sort_rows, DataTable,
};
pub use datepicker::DatePicker;
pub use datetimepicker::DateTimePicker;
pub use datetimerange::DateTimeRange;
pub use dismiss::{
    DismissAxis, DismissDirection, DismissPhase, DismissSpec, DismissState, Dismissable,
    Dismissible,
};
pub use divider::{Divider, DIVIDER_SPACE, DIVIDER_THICKNESS};
pub use dragdrop::{DragSource, DragTarget, Draggable, DropZone};
pub use drawer::{Drawer, DRAWER_WIDTH};
pub use dropdown::Dropdown;
pub use dsl::{button, keyed, spacer, text};
pub use fittedbox::FittedBox;
pub use flex::{Flex, Wrap};
pub use fractional::FractionallySizedBox;
pub use grid::Grid;
pub use hero::{lerp_rect, Hero, HeroSpot};
pub use icon::Icon;
pub use icons::IconName;
pub use image::Image;
pub use ink::{InkStyle, InkWell, Ripples};
pub use inspector::{dump_tree, node_at, paint_overlay as paint_inspector_overlay, InspectorNode};
pub use interaction::{Cursor, InputState, Interaction, Key, KeyResponse, Status, WidgetId};
pub use interactive::{InteractiveView, InteractiveViewer};
pub use kanban::{kanban_slot, Kanban};
pub use kbd::Kbd;
pub use keyed::Keyed;
pub use layoutbuilder::LayoutBuilder;
pub use list::{List, VirtualList};
pub use media::{Edges, MediaQuery};
pub use menu::Menu;
pub use navbar::NavBar;
pub use navigator::Navigator;
pub use navrail::{BottomBar, NavRail};
pub use navscaffold::NavScaffold;
pub use overscroll::{
    cross_axis as glow_cross_axis, edge_for, GlowEdge, OverscrollGlow, ScrollGlows,
};
pub use pageview::{PageSnap, PageView, PagedView};
pub use pagination::Pagination;
pub use paintcache::PaintCache;
pub use physics::{
    page_of, page_target, Ballistic, ScrollMetrics, ScrollPhysics, MAX_FLING_VELOCITY,
    MIN_FLING_VELOCITY,
};
pub use popover::Popover;
pub use portal::{Placement, Portal};
pub use progressbar::ProgressBar;
pub use radio::RadioGroup;
pub use rating::Rating;
pub use refresh::{Refresh, RefreshPhase, RefreshPull, RefreshSpec, Refreshable};
pub use relayout::LayoutCache;
pub use reorder::{reflow_reorder_cards, reflow_reorder_columns};
pub use responsive::{responsive, Responsive};
pub use richtext::RichText;
pub use rotatedbox::RotatedBox;
pub use runtime::{
    spring_ease, spring_step, Anim, Edit, Runtime, ScrollBallistic, ScrollState, ValueAnim,
};
pub use safearea::SafeArea;
pub use scaffold::{fab_button, FabLocation, NavPlacement, Scaffold};
pub use scroll::{Axis, Scroll};
pub use segmented::{
    SegmentedControl, SEGMENTED_BORDER_WIDTH, SEGMENTED_HEIGHT, SEGMENTED_ICON_GAP,
    SEGMENTED_ICON_SIZE, SEGMENTED_PADDING,
};
pub use skeleton::Skeleton;
pub use slider::{RangeSlider, Slider};
pub use spinner::Spinner;
pub use stack::Stack;
pub use stepper::Stepper;
pub use steps::Steps;
pub use switch::Switch;
pub use table::Table;
pub use tabs::{
    Tabs, TabsVariant, TAB_DIVIDER_HEIGHT, TAB_HEIGHT, TAB_INDICATOR_PRIMARY,
    TAB_INDICATOR_SECONDARY, TAB_LABEL_PADDING,
};
pub use text::Text;
pub use textinput::TextInput;
pub use theme::{ColorScheme, TextTheme, Theme};
pub use themed::Themed;
pub use timeline::Timeline;
pub use timepicker::{Endpoint, TimeField, TimePicker, TimeRange};
pub use toast::{SnackbarQueue, Toast, ToastKind};
pub use toasthost::{ToastHost, ToastPosition};
pub use transform::Transform;
pub use tree::Tree;
pub use twopane::TwoPane;
pub use ui::{
    build_ui, build_ui_inspected, collect_ids, find_by_key, find_path, find_widget, subtree_ids,
    FocusDirection, Scrollable, Scrollbar, Ui,
};
pub use widget::{CellFn, ReorderAxis, Widget};
pub use widgettheme::{CardTheme, DividerTheme, DrawerTheme, InkTheme, TabsTheme, WidgetThemes};

// Convenience re-exports for callers.
pub use frus_core::{
    Affine, Alignment, AlignmentDirectional, AlignmentGeometry, Border, BorderRadius,
    BoxDecoration, BoxFit, BoxShadow, ClipShape, Color, FontWeight, ImageData, ImageHandle, Insets,
    InsetsDirectional, LinearGradient, Orientation, Path, PathVerb, Point, Primitive, Rect, Role,
    Scene, Semantics, Size, SizeClass, TextDecoration, TextDirection, TextSpan, TextStyle, Toggled,
    WindowInsets,
};
/// The shared animation layer — physics, curves, driver — see
/// [`frus_core::animation`]. Re-exported here so applications can reach it through
/// `frus_widgets` without depending on `frus-core` directly.
pub use frus_core::{
    AnimationController, ClampedSimulation, Curve, FrictionSimulation, Lerp, Simulation,
    SpringDescription, SpringSimulation, Tolerance, Tween, Velocity, VelocityEstimate,
    VelocityStrategy, VelocityTracker,
};
// `frus_core::Status`, an animation's progress, is renamed so it does not shadow the
// interaction `Status`, which is paint state: hover, press, focus and so on.
pub use frus_core::Status as AnimationStatus;
pub use frus_layout::{Align, Justify};
