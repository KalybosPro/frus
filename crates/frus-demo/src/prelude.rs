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
pub(crate) use std::time::Duration;

pub(crate) use frus_l10n::args;
pub(crate) use frus_shell::{Application, Command, Lifecycle, Subscription};

// `column!` and `row!` are macros: they are exported at frus-widgets' root and have
// to be imported by name in each module, not carried in through a glob.
pub(crate) use frus_widgets::form::{Form, Rule};
pub(crate) use frus_widgets::{
    bar_spacer, button, disabled_content, fab_button, keyed, spacer, text, AlertDialog, Align,
    AnimationController, AppBar, Autocomplete, Axis, BarChart, BottomAppBar, BoxFit, Breadcrumb,
    Card, CarouselView, CellFn, Checkbox, Chip, CircleAvatar, Color, ColorPicker, ConstrainedBox,
    Container, CustomPaint, DataTable, DatePicker, Dismissible, Divider, DragTarget, Draggable,
    DropdownButton, ErrorSummary, Expanded, ExpansionTile, FabLocation, Flex, FontWeight, GridView,
    Hero, Icon, IconButton, Icons, Image, ImageData, ImageHandle, Insets, Justify, Kanban, Kbd,
    LayoutBuilder, LineChart, LinearProgressIndicator, ListView, MenuAnchor, NavigationBar,
    Navigator, Orientation, OverlayPortal, PageView, Pagination, Placement, RadioGroup, Rating,
    Rect, RefreshIndicator, RichText, SafeArea, Scaffold, ScaffoldMessenger, ScrollPhysics,
    SegmentedButton, SingleChildScrollView, Size, SizeClass, SizedBox, Skeleton, Slider, SnackBar,
    SnackBarPosition, SnackBarQueue, SpringDescription, Stack, Stepper, Steps, Switch, TabBar,
    Table, TextField, TextSpan, Theme, Timeline, Tree, TwoPane, Variant, Widget, WindowInsets,
};

// The application's own vocabulary: its state, its messages, and the small modules
// every screen leans on.
pub(crate) use crate::assets::*;
pub(crate) use crate::l10n::*;
pub(crate) use crate::message::Msg;
pub(crate) use crate::model::*;
pub(crate) use crate::parts::*;
pub(crate) use crate::storage::*;
pub(crate) use crate::theme::*;
pub(crate) use crate::update::*;
