//! The frame around the three **sections** — Shop, Search, Cart: their app bar, the
//! drawer, and the navigation between them, a bottom bar on a phone and a rail on a
//! wider window.
//!
//! Each section is its own top-level route, and switching sections is a `go`: the stack
//! becomes that one page, so the back button does not walk through every tab the
//! customer ever touched. Each section page builds this frame around its body.

use frus::{
    Align, AppBar, BuildContext, Callback, Column, DrawerHeader, FontWeight, Icons, ListTile,
    MediaQuery, NavPlacement, RailLabels, Scaffold, SingleChildScrollView, SizeClass, Text, Widget,
};

use crate::common::cart::Cart;
use crate::common::routes::{CART, CHECKOUT, HOME, QUERY, SEARCH, SEARCH_NAME};

/// The widths of a navigation rail, plain and extended (with its labels beside the icons).
const RAIL_WIDTH: f32 = 80.0;
const EXTENDED_RAIL_WIDTH: f32 = 256.0;

/// The three places the bottom bar (or the rail) switches between.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Section {
    Shop,
    Search,
    Cart,
}

impl Section {
    const ALL: [Section; 3] = [Section::Shop, Section::Search, Section::Cart];

    fn index(self) -> usize {
        Section::ALL
            .iter()
            .position(|section| *section == self)
            .unwrap_or(0)
    }

    fn location(self) -> &'static str {
        match self {
            Section::Shop => HOME,
            Section::Search => SEARCH,
            Section::Cart => CART,
        }
    }
}

/// A section's screen: `title` in the app bar (with `actions`), `body` below it, the
/// drawer and the section navigation around it.
///
/// Whether the drawer is open is remembered with a **hook**, `cx.use_state`: called from
/// a page's `build`, it belongs to that page and lives as long as the page does. Every
/// section page calls this once per build, so the hook is always in the same place.
pub fn section_scaffold(
    cx: &BuildContext,
    cart: &Cart,
    section: Section,
    app_bar: AppBar,
    body: impl Widget + 'static,
) -> Box<dyn Widget> {
    let drawer_open = cx.use_state(|| false);
    let router = cx.router();

    // **Switching sections is `go`.** The callback the bar is given makes a `Callback`
    // for the destination that was picked.
    let switch = {
        let router = router.clone();
        move |index: usize| {
            let router = router.clone();
            Callback::new(move || router.go(Section::ALL[index].location()))
        }
    };

    // A phone gets a bottom bar; a wider window a rail on the side, with its labels
    // beside the icons once there is room. The decision is the app's: `Scaffold` draws
    // the navigation where it is told.
    let surface = MediaQuery::of();
    let class = surface.size_class();
    let (placement, rail_width) = match class {
        SizeClass::Compact => (NavPlacement::Bottom, 0.0),
        SizeClass::Medium => (NavPlacement::Rail, RAIL_WIDTH),
        SizeClass::Expanded => (NavPlacement::Rail, EXTENDED_RAIL_WIDTH),
    };
    // An app bar takes the whole window's width unless told otherwise; beside a rail it
    // gets what the rail leaves.
    let app_bar = app_bar.width((surface.size.width - surface.padding.left - rail_width).max(0.0));

    let count = cart.count();
    let mut scaffold = Scaffold::new()
        .background(cx.theme().background)
        // No `leading`: the bar grows the button that opens the drawer by itself, because
        // the scaffold has one (`automatically_imply_leading`, on by default).
        .app_bar(app_bar.build())
        .body(body)
        .drawer(
            // The panel scrolls: a short window has less room than the links need.
            SingleChildScrollView::new().child(drawer(&router, &drawer_open)),
            drawer_open.get(),
            toggle(&drawer_open),
        )
        .nav(section.index(), switch)
        .nav_placement(placement)
        .nav_labels(RailLabels::All)
        .rail(move |rail| rail.extended(class == SizeClass::Expanded))
        .destination(Icons::STOREFRONT, "Shop")
        .destination(Icons::SEARCH, "Search")
        .destination(Icons::SHOPPING_CART, "Cart");
    if count > 0 {
        scaffold = scaffold.badge(count);
    }
    scaffold.build()
}

fn toggle(open: &frus::UseState<bool>) -> Callback {
    let open = open.clone();
    Callback::new(move || open.update(|open| *open = !*open))
}

/// The drawer: links anywhere in the app, including two that show what the router does
/// with a guarded page and with a location nothing answers.
fn drawer(router: &frus::GoRouter, open: &frus::UseState<bool>) -> Column {
    // Each link closes the drawer, then navigates.
    let link = |go: Box<dyn Fn(&frus::GoRouter)>| {
        let (router, open) = (router.clone(), open.clone());
        Callback::new(move || {
            open.set(false);
            go(&router);
        })
    };

    Column::new()
        .cross_axis_alignment(Align::Stretch)
        .child(
            DrawerHeader::new().child(
                Column::new()
                    .spacing(4.0)
                    .cross_axis_alignment(Align::Start)
                    .child(Text::new("Corner Shop").size(22.0).weight(FontWeight::Bold))
                    .child(Text::new("Every link here is a router call").size(13.0)),
            ),
        )
        .child(
            ListTile::new()
                .leading(frus::Icon::new(Icons::STOREFRONT))
                .title("Shop")
                .subtitle("go(\"/\")")
                .on_tap(link(Box::new(|r| r.go(HOME)))),
        )
        .child(
            ListTile::new()
                .leading(frus::Icon::new(Icons::SEARCH))
                .title("Search for shoes")
                .subtitle("go_named(\"search\") with ?q=shoes")
                .on_tap(link(Box::new(|r| {
                    r.go_named(SEARCH_NAME, &[], &[(QUERY, "shoes")])
                }))),
        )
        .child(
            ListTile::new()
                .leading(frus::Icon::new(Icons::SHOPPING_CART))
                .title("Cart")
                .subtitle("go(\"/cart\")")
                .on_tap(link(Box::new(|r| r.go(CART)))),
        )
        .child(
            ListTile::new()
                .leading(frus::Icon::new(Icons::RECEIPT_LONG))
                .title("Checkout")
                .subtitle("Guarded: an empty cart sends you to the cart")
                .on_tap(link(Box::new(|r| r.go(CHECKOUT)))),
        )
        .child(
            ListTile::new()
                .leading(frus::Icon::new(Icons::LINK_OFF))
                .title("An old link")
                .subtitle("go(\"/sale/2019\") — no route answers it")
                .on_tap(link(Box::new(|r| r.go("/sale/2019")))),
        )
}
