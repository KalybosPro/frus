//! [`StickyHeader`]: a section's header that stays at the top of the scroll view while its
//! section is in view (milestone 602).

use frus_core::{Rect, Scene};
use frus_layout::{Align, FlexDirection, Style};

use crate::interaction::Status;
use crate::theme::Theme;
use crate::widget::{FillAxes, Widget};

/// **A section whose header sticks**: in a scroll view, the header stays at the top of the
/// visible part while the rest of its section goes under it, and is pushed off by the next
/// section's header as that one arrives.
///
/// ```
/// use frus_widgets::{Flex, SingleChildScrollView, StickyHeader, Text};
///
/// let _contacts = SingleChildScrollView::<()>::new().child(
///     Flex::column()
///         .child(StickyHeader::new(Text::new("A"), Text::new("Ada\nAlan")))
///         .child(StickyHeader::new(Text::new("B"), Text::new("Barbara\nBjarne"))),
/// );
/// ```
///
/// The header is laid out where a column would put it, above its section, and takes that
/// room. Only where it is painted and where it takes taps moves: never above the section's
/// top, never below its bottom, at the top of what is visible in between. It is drawn over
/// the section, which goes under it. Down a vertical scroll only.
pub struct StickyHeader<Msg = crate::callback::Callback> {
    children: Vec<Box<dyn Widget<Msg>>>,
}

impl<Msg> StickyHeader<Msg> {
    /// A section of `content` under `header`.
    pub fn new(header: impl Widget<Msg> + 'static, content: impl Widget<Msg> + 'static) -> Self {
        Self {
            children: vec![Box::new(header), Box::new(content)],
        }
    }
}

impl<Msg: Clone> Widget<Msg> for StickyHeader<Msg> {
    fn style(&self) -> Style {
        Style {
            flex_direction: FlexDirection::Column,
            align: Align::Stretch,
            flex_shrink: 0.0,
            ..Style::default()
        }
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &self.children
    }

    fn paint(&self, _bounds: Rect, _status: Status, _theme: &Theme, _scene: &mut Scene) {}

    fn on_click(&self) -> Option<Msg> {
        None
    }

    fn sticky_header(&self) -> bool {
        true
    }

    /// The width: a section is a band across its list.
    fn fill_axes(&self, _theme: &Theme) -> FillAxes {
        FillAxes {
            horizontal: true,
            vertical: false,
        }
    }

    fn debug_name(&self) -> &'static str {
        "StickyHeader"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::interaction::WidgetId;
    use crate::{build_ui, Container, Flex, Runtime, SingleChildScrollView};
    use frus_core::{Color, Point, Primitive, Size};

    const RED: Color = Color::rgb(1.0, 0.0, 0.0);
    const BLUE: Color = Color::rgb(0.0, 0.0, 1.0);
    const GREY: Color = Color::rgb(0.5, 0.5, 0.5);

    /// Two 120-px sections, each a 20-px header over a 100-px grey body, in a 200-px scroll.
    /// The red header says 1 when tapped, the bodies 9.
    fn sections() -> SingleChildScrollView<u8> {
        // Through a key, as a list's rows usually are: the wrapper passes the question on.
        let section = |color: Color, says: u8| {
            crate::keyed(
                says,
                StickyHeader::new(
                    Container::new().height(20.0).color(color).on_click(says),
                    Container::new().height(100.0).color(GREY).on_click(9),
                ),
            )
        };
        SingleChildScrollView::new()
            .width(200.0)
            .height(200.0)
            .child(
                Flex::column()
                    .child(section(RED, 1))
                    .child(section(BLUE, 2))
                    .child(Container::new().height(300.0)),
            )
    }

    /// Where each header is painted, scrolled down by `offset`.
    fn headers(offset: f32) -> (f32, f32) {
        let mut runtime = Runtime::default();
        runtime.scroll.insert(WidgetId::ROOT, (0.0, offset));
        let ui = build_ui(
            &sections(),
            Size::new(200.0, 200.0),
            &runtime,
            &Theme::dark(),
        );
        let top = |color: Color| {
            ui.scene()
                .primitives()
                .iter()
                .find_map(|p| match p {
                    Primitive::Rect { rect, color: c, .. } if *c == color => Some(rect.y),
                    _ => None,
                })
                .expect("the header")
        };
        (top(RED), top(BLUE))
    }

    /// **At rest**, each header is where the column put it.
    #[test]
    fn at_rest_the_headers_are_where_they_were_laid_out() {
        assert_eq!(headers(0.0), (0.0, 120.0));
    }

    /// **Scrolled into its section**, the header stays at the top while the next comes up.
    #[test]
    fn a_header_sticks_while_its_section_is_in_view() {
        assert_eq!(headers(50.0), (0.0, 70.0));
    }

    /// **At the end of its section**, the next header pushes it off, and then sticks itself.
    #[test]
    fn the_next_header_pushes_it_off() {
        assert_eq!(headers(110.0), (-10.0, 10.0));
        assert_eq!(headers(130.0), (-30.0, 0.0));
    }

    /// **It is drawn over its section**, and takes the tap there: the body under a stuck
    /// header does not.
    #[test]
    fn a_stuck_header_is_over_its_section_and_takes_the_tap() {
        let mut runtime = Runtime::default();
        runtime.scroll.insert(WidgetId::ROOT, (0.0, 50.0));
        let ui = build_ui(
            &sections(),
            Size::new(200.0, 200.0),
            &runtime,
            &Theme::dark(),
        );
        let order: Vec<Color> = ui
            .scene()
            .primitives()
            .iter()
            .filter_map(|p| match p {
                Primitive::Rect { color, .. } if *color == RED || *color == GREY => Some(*color),
                _ => None,
            })
            .collect();
        assert_eq!(order.first(), Some(&GREY), "the body first: {order:?}");
        assert_eq!(order.get(1), Some(&RED), "then its header: {order:?}");
        let hit = ui.hit(Point::new(100.0, 10.0)).expect("something takes it");
        assert_eq!(ui.msg_for(hit), Some(1));
        let hit = ui.hit(Point::new(100.0, 40.0)).expect("the body below");
        assert_eq!(ui.msg_for(hit), Some(9));
    }

    /// **As the rows of a list**, the same: a list lays its rows out one by one, and the
    /// header still sticks and is pushed off.
    #[test]
    fn a_header_sticks_in_a_list() {
        let list = crate::ListView::<()>::new(4, 120.0, |i| {
            StickyHeader::new(
                Container::<()>::new()
                    .height(20.0)
                    .color(if i == 0 { RED } else { BLUE }),
                Container::<()>::new().height(100.0).color(GREY),
            )
        })
        .width(200.0)
        .height(200.0);
        let mut runtime = Runtime::default();
        runtime.scroll.insert(WidgetId::ROOT, (0.0, 50.0));
        let ui = build_ui(&list, Size::new(200.0, 200.0), &runtime, &Theme::dark());
        let tops: Vec<(Color, f32)> = ui
            .scene()
            .primitives()
            .iter()
            .filter_map(|p| match p {
                Primitive::Rect { rect, color, .. } if *color == RED || *color == BLUE => {
                    Some((*color, rect.y))
                }
                _ => None,
            })
            .collect();
        assert!(tops.contains(&(RED, 0.0)), "{tops:?}");
        assert!(tops.contains(&(BLUE, 70.0)), "{tops:?}");
    }
}
