use frus::{
    Align, AppBar, Axis, Brightness, Center, Column, Container, Expanded, FontFamily, Scaffold,
    SingleChildScrollView, SizedBox, State, StateContext, StatefulWidget, TabBar, Text, Widget,
};

use crate::common::{catalog::demo, colors::code_background};
use crate::features::catalog::states::demostate::DemoState;

/// One widget's page: the live example in one tab, the file it is written in in the
/// other.
pub struct DemoPage {
    id: String,
    /// The tab it opens on: `/widget/<id>?tab=code` opens on the code.
    initial_tab: usize,
}

impl DemoPage {
    pub fn new(id: &str, tab: Option<&str>) -> Self {
        Self {
            id: id.to_string(),
            initial_tab: usize::from(tab == Some("code")),
        }
    }
}

impl StatefulWidget for DemoPage {
    type State = DemoState;

    fn create_state(&self) -> DemoState {
        DemoState {
            tab: self.initial_tab,
        }
    }
}

impl State for DemoState {
    type Widget = DemoPage;

    fn build(&self, cx: &StateContext<Self>) -> Box<dyn Widget> {
        let theme = cx.theme();
        let id = cx.widget().id.clone();
        let Some((_, demo)) = demo(&id) else {
            return Box::new(Center::new(Text::new(format!("No demo `{id}`"))));
        };

        // Only the selected tab's panel is built.
        let example = || {
            SingleChildScrollView::new().padding(20.0).child(
                Column::new()
                    .spacing(20.0)
                    .cross_axis_alignment(Align::Start)
                    .child(Text::new(demo.summary).color(theme.muted))
                    .child((demo.build)()),
            )
        };

        let dark = theme.brightness() == Brightness::Dark;
        let code = || {
            Container::new().color(code_background(dark)).child(
                SingleChildScrollView::new()
                    .axis(Axis::Both)
                    .padding(16.0)
                    .child(
                        Text::new(demo.source)
                            .family(FontFamily::Monospace)
                            .size(13.0)
                            .no_wrap()
                            .selectable(),
                    ),
            )
        };

        // The tab bar is only the strip; the selected panel sits under it in an `Expanded`,
        // so it fills the rest of the page. (A tab set that holds its own panels is as tall
        // as their content, and a swipeable one builds its pages during layout — which a
        // demo made of components cannot be.)
        let strip = TabBar::new(
            self.tab,
            cx.handler(|state: &mut DemoState, tab| state.select_tab(tab)),
        )
        .tab("Example", SizedBox::empty())
        .tab("Code", SizedBox::empty());
        let panel: Box<dyn Widget> = match self.tab {
            0 => Box::new(example()),
            _ => Box::new(code()),
        };
        // Stretched: the panel is exactly as wide as the page, so a long line of code scrolls
        // sideways instead of widening the page.
        let body = Column::new()
            .cross_axis_alignment(Align::Stretch)
            .child(strip)
            .child(Expanded::new(panel));

        Scaffold::new()
            .background(theme.background)
            .app_bar(AppBar::new().title(Text::new(demo.name)).build())
            .body(body)
            .build()
    }
}
