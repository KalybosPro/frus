//! `frus-widgets` — frus's declarative widget tree.
//!
//! An interface is described with composable widgets ([`Container`], [`Flex`],
//! [`Text`], [`TextField`], [`SingleChildScrollView`]) generic over a message type `Msg`, in the
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
mod banner;
mod barrier;
mod baseline;
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
mod dialog;
/// The **disabled state** every control shares: two colours and a four-part contract.
pub mod disabled;
/// Swipe-to-dismiss: the [`dismiss::Dismissible`] widget and its retained state.
pub mod dismiss;
mod divider;
mod dragdrop;
mod drawer;
mod dropdown;
mod dsl;
mod expanded;
mod filters;
mod fittedbox;
mod flex;
mod focus;
/// Form validation, pure and application-side: [`form::Rule`] and [`form::Form`].
pub mod form;
pub use form::ErrorSummary;
mod fractional;
mod grid;
mod hero;
mod icon;
mod iconbutton;
mod icons;
mod image;
/// The software keyboard's vocabulary: which keys, and what the action key does.
pub mod ime;
mod ink;
mod inspector;
mod interaction;
mod interactive;
mod kanban;
mod kbd;
mod keyed;
mod layoutbuilder;
mod list;
mod listtile;
/// The words the framework itself says: [`localizations::Localizations`] and [`localizations::of`].
pub mod localizations;
mod media;
mod mediascope;
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
mod placeholder;
mod popover;
mod portal;
mod positioned;
mod progressbar;
mod radio;
mod rating;
/// Pull-to-refresh: the [`refresh::RefreshIndicator`] widget and its retained pull.
pub mod refresh;
mod relayout;
mod reorder;
mod responsive;
mod richtext;
mod rotatedbox;
mod rowcolumn;
mod runtime;
mod safearea;
mod scaffold;
mod scaffoldinfo;
mod scroll;
mod segmented;
/// Stating what a widget **is** from outside it: the [`semantics::Semantics`] wrapper.
pub mod semantics;
mod shortcuts;
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
mod themebuilder;
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
mod widgetstate;
mod widgettheme;

pub use alert::{Alert, AlertKind};
pub use animated::{AnimatedContainer, AnimatedOpacity, Opacity};
pub use appbar::{platform_centers_title, AppBar, APP_BAR_HEIGHT, APP_BAR_MAX_TITLE_SCALE};
pub use aspectratio::AspectRatio;
pub use autocomplete::Autocomplete;
pub use avatar::CircleAvatar;
pub use badge::Badge;
pub use banner::{MaterialBanner, BANNER_ELEVATION, BANNER_MIN_ACTION_BAR_HEIGHT};
pub use barrier::{
    AbsorbPointer, ExcludeSemantics, IgnorePointer, ModalBarrier, Offstage, Visibility,
};
pub use baseline::{Baseline, IgnoreBaseline};
pub use bottomappbar::{bar_spacer, notched_outline, BottomAppBar};
pub use bottomsheet::BottomSheet;
pub use breadcrumb::Breadcrumb;
pub use button::{
    Button, Variant, BUTTON_BORDER_WIDTH, BUTTON_ELEVATION, BUTTON_HEIGHT, BUTTON_MIN_WIDTH,
    BUTTON_PADDING, BUTTON_TEXT_PADDING,
};
pub use card::{Card, CardVariant, CARD_ELEVATION, CARD_MARGIN};
pub use carousel::CarouselView;
pub use chart::{BarChart, LineChart};
pub use checkbox::Checkbox;
pub use chip::{
    Chip, CHIP_BORDER_WIDTH, CHIP_HEIGHT, CHIP_ICON_SIZE, CHIP_LABEL_PADDING, CHIP_PADDING,
    CHIP_RADIUS,
};
pub use clip::{ClipOval, ClipPath, ClipRRect};
pub use collapsible::{ControlAffinity, ExpansionTile};
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
pub use dialog::{
    ActionsAlignment, AlertDialog, Dialog, SimpleDialog, SimpleDialogOption, DIALOG_ELEVATION,
    DIALOG_INSET_PADDING, DIALOG_MIN_WIDTH, DIALOG_RADIUS,
};
pub use disabled::{
    disabled_container, disabled_content, disabled_mark, over_surface, DISABLED_CONTAINER_OPACITY,
    DISABLED_CONTENT_OPACITY,
};
pub use dismiss::{
    DismissAxis, DismissDirection, DismissPhase, DismissSpec, DismissState, Dismissable,
    Dismissible,
};
pub use divider::{Divider, DIVIDER_SPACE, DIVIDER_THICKNESS};
pub use dragdrop::{DragSource, DragTarget, Draggable, DropZone};
pub use drawer::{Drawer, DRAWER_RADIUS, DRAWER_WIDTH};
pub use dropdown::DropdownButton;
pub use dsl::{button, expanded, flexible, keyed, spacer, text};
pub use expanded::{Expanded, FlexFit, Flexible};
pub use filters::{BackdropFilter, BackdropGroup, ColorFiltered, ImageFiltered, ShaderMask};
pub use fittedbox::FittedBox;
pub use flex::{Flex, Wrap};
pub use focus::{
    ExcludeFocus, ExcludeFocusTraversal, Focus, FocusTraversalGroup, FocusTraversalOrder,
};
pub use fractional::FractionallySizedBox;
pub use grid::GridView;
pub use hero::{lerp_rect, Hero, HeroSpot};
pub use icon::Icon;
pub use iconbutton::{
    IconButton, IconButtonVariant, ICON_BUTTON_BORDER_WIDTH, ICON_BUTTON_ICON_SIZE,
    ICON_BUTTON_SIZE,
};
pub use icons::Icons;
pub use image::{Image, State as ImageState};
pub use ime::{Capitalization, Ime, KeyboardType, TextInputAction};
pub use ink::{InkStyle, InkWell, Ripples};
pub use inspector::{dump_tree, node_at, paint_overlay as paint_inspector_overlay, InspectorNode};
pub use interaction::{Cursor, InputState, Interaction, Key, KeyResponse, Status, WidgetId};
pub use interactive::{InteractiveView, InteractiveViewer};
pub use kanban::{kanban_slot, Kanban};
pub use kbd::Kbd;
pub use keyed::Keyed;
pub use layoutbuilder::LayoutBuilder;
pub use list::{ListView, VirtualList};
pub use listtile::{
    ListTile, LIST_TILE_DENSE_HEIGHTS, LIST_TILE_HEIGHTS, LIST_TILE_MIN_LEADING_WIDTH,
    LIST_TILE_MIN_VERTICAL_PADDING, LIST_TILE_PADDING_END, LIST_TILE_PADDING_START,
    LIST_TILE_TITLE_GAP,
};
pub use localizations::{English, Localizations};
pub use media::{
    Accessibility, AccessibilityOverrides, Brightness, Edges, MediaQuery, SurfaceGuard,
};
pub use mediascope::MediaScope;
pub use menu::PopupMenuButton;
pub use navbar::NavigationBar;
pub use navigator::Navigator;
pub use navrail::{BottomBar, NavigationRail, RailLabels};
pub use navscaffold::NavScaffold;
pub use overscroll::{
    cross_axis as glow_cross_axis, edge_for, GlowEdge, OverscrollGlow, ScrollGlows,
};
pub use pageview::{PageSnap, PageView, PagedView};
pub use pagination::Pagination;
pub use paintcache::PaintCache;
pub use physics::{
    page_of, page_target, Ballistic, ScrollMetrics, ScrollPhysics, Scrollbars, MAX_FLING_VELOCITY,
    MIN_FLING_VELOCITY,
};
pub use placeholder::{Placeholder, PLACEHOLDER_COLOR, PLACEHOLDER_FALLBACK, PLACEHOLDER_STROKE};
pub use popover::MenuAnchor;
pub use portal::{OverlayPortal, Placement};
pub use positioned::{Positioned, Positioning};
pub use progressbar::LinearProgressIndicator;
pub use radio::RadioGroup;
pub use rating::Rating;
pub use refresh::{RefreshIndicator, RefreshPhase, RefreshPull, RefreshSpec, Refreshable};
pub use relayout::LayoutCache;
pub use reorder::{reflow_reorder_cards, reflow_reorder_columns};
pub use responsive::{responsive, Responsive};
pub use richtext::RichText;
pub use rotatedbox::RotatedBox;
pub use rowcolumn::{Column, MainAxisSize, Row, VerticalDirection};
pub use runtime::{
    spring_ease, spring_step, Anim, Edit, Runtime, ScrollBallistic, ScrollState, ScrollbarFade,
    ValueAnim,
};
pub use safearea::SafeArea;
pub use scaffold::{fab_button, FabLocation, NavPlacement, Scaffold};
pub use scaffoldinfo::{ScaffoldGuard, ScaffoldInfo, ScaffoldScope};
pub use scroll::{Axis, SingleChildScrollView};
pub use segmented::{
    SegmentedButton, SEGMENTED_BORDER_WIDTH, SEGMENTED_HEIGHT, SEGMENTED_ICON_GAP,
    SEGMENTED_ICON_SIZE, SEGMENTED_PADDING,
};
pub use semantics::{Description, Semantics};
pub use shortcuts::{
    ActionListener, Actions, CallbackShortcuts, FocusableActionDetector, Intent, KeyStroke,
    KeyboardListener, ShortcutKey, Shortcuts,
};
pub use skeleton::Skeleton;
pub use slider::{RangeSlider, Slider};
pub use spinner::CircularProgressIndicator;
pub use stack::{Stack, StackFit};
pub use stepper::Stepper;
pub use steps::Steps;
pub use switch::Switch;
pub use table::Table;
pub use tabs::{
    TabAlignment, TabBar, TabBarVariant, TAB_DIVIDER_HEIGHT, TAB_HEIGHT, TAB_ICON_GAP,
    TAB_ICON_HEIGHT, TAB_ICON_SIZE, TAB_INDICATOR_PRIMARY, TAB_INDICATOR_SECONDARY,
    TAB_LABEL_PADDING, TAB_START_OFFSET,
};
pub use text::Text;
pub use textinput::{
    TextField, TextFieldStyle, TextFieldVariant, FIELD_BORDER_WIDTH,
    FIELD_DENSE_OUTLINED_PADDING_BOTTOM, FIELD_DENSE_OUTLINED_PADDING_TOP, FIELD_DENSE_PADDING_Y,
    FIELD_DISABLED_OPACITY, FIELD_FOCUSED_BORDER_WIDTH, FIELD_GAP, FIELD_ICON_SIZE,
    FIELD_LABEL_SCALE, FIELD_NOTCH_GAP, FIELD_OUTLINED_PADDING_BOTTOM, FIELD_OUTLINED_PADDING_TOP,
    FIELD_PADDING_X, FIELD_PADDING_Y, FIELD_RADIUS, FIELD_SUB_SIZE, FIELD_TEXT_SIZE,
};
pub use theme::{
    ColorScheme, TapTarget, TextTheme, Theme, ThemeMode, MIN_TAP_TARGET, SHRUNK_TAP_TARGET,
};
pub use themebuilder::ThemeBuilder;
pub use themed::Themed;
pub use timeline::Timeline;
pub use timepicker::{Endpoint, TimeField, TimePicker, TimeRange};
pub use toast::{SnackBar, SnackBarBehavior, SnackBarKind, SnackBarQueue};
pub use toasthost::{ScaffoldMessenger, SnackBarPosition};
pub use transform::Transform;
pub use tree::Tree;
pub use twopane::TwoPane;
pub use ui::{
    build_deferred, build_ui, build_ui_inspected, collect_ids, find_by_key, find_path, find_widget,
    subtree_ids, FocusDirection, Focusable, KeepVisible, Scrollable, Scrollbar, Ui,
};
pub use widget::{CellFn, FillAxes, FilterContext, ReorderAxis, Widget};
pub use widgetstate::{StateFilter, WidgetState, WidgetStateProperty, WidgetStates};
// **Every** one of them, and not only the ones a doc link happened to need. A theme
// struct that is `pub` inside a private module is public and unreachable: callers can
// still set `theme.widgets.button.background`, because the field's type is inferred, but
// they cannot name the type — so `AppBar::icon_theme` took an `IconTheme` no caller
// could build from milestone 396 until now. A property that ships unusable is a property
// that did not ship.
pub use widgettheme::resolve_shape;
pub use widgettheme::{
    AppBarTheme, BadgeTheme, ButtonTheme, CardTheme, CheckboxTheme, ChipTheme, DefaultTextStyle,
    DividerTheme, DrawerTheme, IconButtonTheme, IconTheme, InkTheme, RadioTheme, SegmentedTheme,
    SliderTheme, SwitchTheme, TabBarTheme, TextFieldTheme, WidgetThemes,
};

// Convenience re-exports for callers.
/// Installing the reader's font size for a whole frame — see
/// [`frus_core::install_text_scale`]. Re-exported for the shell, which has to hold it
/// across the build, the layout **and** the paint.
pub use frus_core::{install_text_scale, TextScaleGuard};
pub use frus_core::{
    Affine, Alignment, AlignmentDirectional, AlignmentGeometry, Backdrop, BlendMode, Border,
    BorderRadius, BoxDecoration, BoxFit, BoxShadow, ClipShape, Color, ColorFilter, FontWeight,
    FractionalMask, ImageData, ImageFilter, ImageHandle, Insets, InsetsDirectional, LinearGradient,
    Orientation, Path, PathVerb, Point, Primitive, Rect, Role, Scene, SemanticsProperties, Size,
    SizeClass, TextAlign, TextDecoration, TextDirection, TextOverflow, TextSpan, TextStyle,
    Toggled, WindowInsets,
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

/// The image stores and the two things a host has to name: a decoder is chosen by the
/// `images` feature, and the **fetcher** is registered by whoever has a network. See
/// [`Image::network`].
pub use frus_core::{images_in_flight, set_image_fetcher, ImageFetcher};
pub use frus_layout::{Align, AlignContent, FlexDirection, Justify, Overflowing, Side};
