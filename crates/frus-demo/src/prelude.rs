//! What every module of this application needs, in one import.
//!
//! Naming the same thirty widgets at the top of every screen tells the reader
//! nothing, so they are named once here and each module writes
//! `use crate::prelude::*;`. It is the same idea as a UI toolkit shipping a single
//! import that brings in its whole widget set.
//!
//! It is for what is genuinely common. A screen needing one unusual widget still
//! imports that one by name — the prelude is here to remove noise, not to hide where
//! things come from.

pub(crate) use std::path::{Path, PathBuf};
pub(crate) use std::rc::Rc;

// `column!` and `row!` are macros: they are exported at frus-widgets' root and have
// to be imported by name in each module, not carried in through a glob.
pub(crate) use frus_widgets::form::{Form, Rule};
pub(crate) use frus_widgets::{
    bar_spacer, button, disabled_content, eased, fab_button, keyed, spacer, text, AboutDialog,
    AboutListTile, Alert, Align, AnimatedDefaultTextStyle, AnimatedFractionallySizedBox,
    AnimatedIcons, AnimatedPadding, AnimatedPositioned, AnimatedSlide, AnimatedSwitcher, AppBar,
    Autocomplete, Axis, BarChart, BottomAppBar, BoxFit, Breadcrumb, BuildContext, Callback, Card,
    CarouselView, CellFn, Checkbox, Chip, CircleAvatar, Color, ColorPicker, Component,
    ConstrainedBox, Container, Curve, CustomPaint, DataTable, DatePicker,
    DefaultTextStyleTransition, Dismissible, Divider, DragTarget, Draggable, DropdownButton,
    DropdownMenu, ErrorSummary, Expanded, ExpansionTile, FabLocation, FadeTransition, Flex,
    FontWeight, GoRoute, GoRouter, GridView, Hero, Icon, IconButton, Icons, Image, ImageData,
    ImageHandle, ImageIcon, Justify, Kanban, Kbd, LayoutBuilder, LicensePage, LineChart,
    LinearProgressIndicator, ListView, ListWheel, MediaQuery, MenuAnchor, NavigationBar,
    NavigationDestination, OverlayPortal, PageView, Pagination, Placement, Positioned, RadioGroup,
    Rating, Rect, RefreshIndicator, ReorderGrab, ReorderableList, RichText, SafeArea, Scaffold,
    ScaffoldMessenger, ScaleTransition, ScrollPhysics, ScrollPosition, ScrollTo, SegmentedButton,
    SingleChildScrollView, Size, SizeClass, SizedBox, Skeleton, SlideFrom, SlideTransition, Slider,
    SnackBar, SnackBarPosition, Stack, State, StateContext, StatefulWidget, StatelessWidget,
    Stepper, Steps, Switch, TabBar, TabPageSelector, Table, TextField, TextSpan, TextStyle, Theme,
    Timeline, Tree, TwoPane, Variant, Widget,
};

// The application's own vocabulary: its shared state, its data, and the small modules
// every screen leans on.
pub(crate) use crate::assets::*;
pub(crate) use crate::demo::Demo;
pub(crate) use crate::l10n::*;
pub(crate) use crate::model::*;
pub(crate) use crate::parts::*;
pub(crate) use crate::theme::*;
