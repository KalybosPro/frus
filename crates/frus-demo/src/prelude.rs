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
    bar_spacer, button, fab_button, keyed, spacer, text, Alert, Align, AnimationController, AppBar,
    Autocomplete, Avatar, Axis, BarChart, BottomAppBar, BoxFit, Breadcrumb, Card, Carousel, CellFn,
    Checkbox, Chip, Collapsible, Color, ColorPicker, Container, CustomPaint, DataTable, DatePicker,
    Dismissible, Divider, DragTarget, Draggable, Dropdown, ErrorSummary, FabLocation, Flex,
    FontWeight, Grid, Hero, Icon, IconName, Image, ImageData, ImageHandle, Insets, Justify, Kanban,
    Kbd, LayoutBuilder, LineChart, List, NavBar, Navigator, Orientation, PageView, Pagination,
    Placement, Popover, Portal, ProgressBar, RadioGroup, Rating, Rect, Refresh, RichText, Scaffold,
    Scroll, ScrollPhysics, SegmentedControl, Size, SizeClass, SizedBox, Skeleton, Slider,
    SnackbarQueue, SpringDescription, Stack, Stepper, Steps, Switch, Table, Tabs, TextInput,
    TextSpan, Theme, Timeline, Toast, ToastHost, ToastPosition, Tree, TwoPane, Variant, Widget,
    WindowInsets,
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
