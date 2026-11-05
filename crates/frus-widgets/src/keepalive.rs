// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 KalybosPro <kalyboskokou1@gmail.com>

//! [`KeepAlive`]: a **transparent** wrapper by which a row of a list built on demand asks
//! the list to keep its state while it is scrolled out of view (milestone 748).
//!
//! A [`ListView::builder`](crate::ListView::builder) only builds the rows in view. What the
//! components of a row keep — a [`State`](crate::State), the hooks of a function component
//! — is let go when the row leaves, unless the row asks to be kept: with this wrapper,
//! anywhere in the row, or with [`BuildContext::keep_alive`](crate::BuildContext::keep_alive)
//! from a component in it. The list's own default is
//! [`ListView::keep_alive`](crate::ListView::keep_alive).

use crate::widget::Widget;

/// Asks the list whose row this is in to keep the row's state while it is out of view —
/// or, with [`KeepAlive::keep`]`(false)`, declines the list's default to keep it.
///
/// ```
/// use frus_widgets::{KeepAlive, ListView, Text};
///
/// // Only the pinned rows keep what they hold when scrolled away.
/// let list: ListView = ListView::builder(100, |i| {
///     KeepAlive::new(Text::new(format!("row {i}"))).keep(i % 10 == 0)
/// });
/// # let _ = list;
/// ```
pub struct KeepAlive<Msg = crate::callback::Callback> {
    keep: bool,
    inner: Box<dyn Widget<Msg>>,
}

impl<Msg> KeepAlive<Msg> {
    /// Keeps the row `inner` is in alive while it is out of view.
    pub fn new(inner: impl Widget<Msg> + 'static) -> Self {
        Self {
            keep: true,
            inner: Box::new(inner),
        }
    }

    /// Whether to keep it: `false` asks for the row to be let go when it leaves, whatever
    /// the list keeps by default — unless something else in the row asks to keep it.
    pub fn keep(mut self, keep: bool) -> Self {
        self.keep = keep;
        self
    }

    /// Keeping a row is not a box: the child's own, unchanged.
    fn restyle(&self, base: frus_layout::Style) -> frus_layout::Style {
        base
    }
}

crate::transparent::forward_transparent!(expand KeepAlive {
    /// The one thing it does, as the row is built: it asks, then the child is built.
    fn expand(
        &self,
        id: crate::interaction::WidgetId,
        runtime: &crate::runtime::Runtime,
        theme: &crate::theme::Theme,
    ) {
        runtime.states.want_keep_alive(self.keep);
        self.inner.expand(id, runtime, theme)
    }

    /// Forwarded: keeping a row alive is not an identity — `KeepAlive(Keyed(..))` is found
    /// by the key.
    fn key(&self) -> Option<u64> {
        self.inner.key()
    }
    fn positioned(&self) -> Option<crate::positioned::Positioning> {
        self.inner.positioned()
    }
    fn theme_override(
        &self,
        inherited: &crate::theme::Theme,
    ) -> Option<Box<crate::theme::Theme>> {
        self.inner.theme_override(inherited)
    }
    fn media_override(&self, inherited: crate::MediaQuery) -> Option<crate::MediaQuery> {
        self.inner.media_override(inherited)
    }
    fn scaffold_override(&self) -> Option<crate::ScaffoldInfo> {
        self.inner.scaffold_override()
    }
    fn scroll_behavior_override(&self) -> Option<crate::ScrollBehavior> {
        self.inner.scroll_behavior_override()
    }
    fn autofill_group(&self) -> bool {
        self.inner.autofill_group()
    }
    fn area_toolbar(
        &self,
        context: crate::ToolbarContext,
    ) -> Option<Option<&dyn crate::widget::Widget<Msg>>> {
        self.inner.area_toolbar(context)
    }
});
