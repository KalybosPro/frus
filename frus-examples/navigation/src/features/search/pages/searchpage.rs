use frus::{
    Align, AppBar, Callback, Column, GoRouter, Icons, SingleChildScrollView, State, StateContext,
    StatefulWidget, StatelessWidget, Text, TextField, Widget,
};

use crate::common::{
    cart::Cart,
    routes::{PRODUCT_NAME, SEARCH_NAME},
    shared::empty_state,
    shell::{section_scaffold, Section},
};
use crate::features::home::widgets::product_card::ProductCard;
use crate::features::search::{
    states::searchstate::{query_params, results, SearchState, Sort},
    widgets::sort_picker::SortPicker,
};

/// Search (`/search?q=shoes&sort=price`). The router reads the query and **configures**
/// the page with it: the address is the source of truth, and the page writes to it.
pub(crate) struct SearchPage {
    cart: Cart,
    query: String,
    sort: Sort,
}

impl SearchPage {
    pub(crate) fn new(cart: Cart, query: &str, sort: Sort) -> Self {
        Self {
            cart,
            query: query.to_string(),
            sort,
        }
    }
}

impl StatefulWidget for SearchPage {
    type State = SearchState;

    fn create_state(&self) -> SearchState {
        SearchState::new(&self.query)
    }
}

/// Writes a search into the address.
///
/// A `go` to the same route keeps the same page — the router recognises it by its path,
/// not its query — so the field keeps its focus and its state while the address follows
/// every keystroke.
fn show(router: &GoRouter, text: &str, sort: Sort) {
    let params = query_params(text, sort);
    let params: Vec<(&str, &str)> = params.iter().map(|(k, v)| (*k, v.as_str())).collect();
    router.go_named(SEARCH_NAME, &[], &params);
}

impl State for SearchState {
    type Widget = SearchPage;

    /// The address can change without this page typing anything — the drawer's "Search
    /// for shoes" link, the back button on the web. When the query it is configured with
    /// differs from the field, the address wins.
    fn did_update_widget(&mut self, old: &SearchPage, cx: &StateContext<Self>) {
        let query = &cx.widget().query;
        if *query != old.query && query.trim() != self.text.trim() {
            self.text = query.clone();
        }
    }

    fn build(&self, cx: &StateContext<Self>) -> Box<dyn Widget> {
        let router = cx.router();
        let page = cx.widget();
        let sort = page.sort;

        let field = TextField::new(self.text.as_str())
            .placeholder("Search the shop")
            .prefix_icon(Icons::SEARCH)
            .on_input(cx.handler({
                let router = router.clone();
                move |s: &mut SearchState, text: String| {
                    s.text = text;
                    show(&router, &s.text, sort);
                }
            }));

        let picker = SortPicker::new(sort, {
            let router = router.clone();
            let text = self.text.clone();
            move |picked| {
                let (router, text) = (router.clone(), text.clone());
                Callback::new(move || show(&router, &text, picked))
            }
        });

        // **`push_named`, not `go`**: the product goes *on top of* this search, so the
        // back button returns here with the query and the order as they were. A `go`
        // would have replaced the stack with [home, product].
        let found = results(&self.text, sort);
        let list: Box<dyn Widget> = if found.is_empty() {
            Box::new(empty_state(
                cx,
                Icons::SEARCH_OFF,
                format!("Nothing matches \"{}\"", self.text.trim()),
                None,
            ))
        } else {
            let list = found
                .into_iter()
                .fold(Column::new().spacing(8.0), |list, product| {
                    let router = router.clone();
                    let id = product.id.to_string();
                    let open =
                        Callback::new(move || router.push_named(PRODUCT_NAME, &[("id", &id)], &[]));
                    list.child(ProductCard::new(product, open).into_widget())
                });
            Box::new(list)
        };

        let body = SingleChildScrollView::new().padding(16.0).child(
            Column::new()
                .spacing(16.0)
                .cross_axis_alignment(Align::Stretch)
                .child(field)
                .child(picker.into_widget())
                .child_boxed(list),
        );

        section_scaffold(
            cx,
            &page.cart,
            Section::Search,
            AppBar::new().title(Text::new("Search")),
            body,
        )
    }
}
