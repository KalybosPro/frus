//! [`FloatingActionButton`]: the one action a screen is **for**, sitting above it.

use frus_core::{BorderRadius, Color, Point, Rect, Scene, ShapeBorder, TextStyle};
use frus_layout::{Dimension, Style};

use crate::disabled::{disabled_container, disabled_content};
use crate::icons::Icons;
use crate::interaction::Status;
use crate::theme::Theme;
use crate::widget::Widget;

/// The grid an icon's path is drawn on, so a glyph can be scaled to any size.
const ICON_GRID: f32 = 24.0;

/// The four sizes the reference draws (`floating_action_button.dart:783`), each with its
/// own box, its own corner and its own icon.
///
/// They are one enum rather than four constructors' worth of fields because every one of
/// those three numbers follows from **which** of the four it is, and a size that took
/// them separately would let a caller build a large button with a small button's corner.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub enum FabSize {
    /// 56 across, 16 at the corners, a 24 icon. The one meant by "the button".
    #[default]
    Regular,
    /// 40 and 12, with the same 24 icon — for a screen that already has a bigger one, or
    /// a surface too small to give up 56.
    Small,
    /// 96 and 28, with a 36 icon: a single action on a large screen.
    Large,
    /// 56 tall and as wide as its words, 16 at the corners. An icon **and** a label,
    /// for an action whose glyph would not say what it does.
    Extended,
}

impl FabSize {
    /// The button's side, or its height when it is extended.
    pub const fn extent(self) -> f32 {
        match self {
            FabSize::Small => 40.0,
            FabSize::Large => 96.0,
            FabSize::Regular | FabSize::Extended => 56.0,
        }
    }

    /// The corner the reference gives it (`floating_action_button.dart:816`).
    pub const fn radius(self) -> f32 {
        match self {
            FabSize::Small => 12.0,
            FabSize::Large => 28.0,
            FabSize::Regular | FabSize::Extended => 16.0,
        }
    }

    /// How big the glyph inside is (`floating_action_button.dart:824`).
    pub const fn icon_size(self) -> f32 {
        match self {
            FabSize::Large => 36.0,
            _ => 24.0,
        }
    }
}

/// The room either side of an extended button's content
/// (`floating_action_button.dart:831`): twenty at the end, and sixteen at the start when
/// there is a glyph before the words.
const EXTENDED_PAD: f32 = 20.0;
const EXTENDED_PAD_WITH_ICON: f32 = 16.0;
/// The gap between that glyph and the words (`floating_action_button.dart:798`).
const EXTENDED_GAP: f32 = 8.0;

/// How far off the page the button sits, at rest and under a pointer
/// (`floating_action_button.dart:778`).
const ELEVATION: f32 = 6.0;
const HOVER_ELEVATION: f32 = 8.0;

/// **The one action a screen is for.**
///
/// ```
/// # use frus_widgets::{FloatingActionButton, Icons};
/// # #[derive(Clone)] enum Msg { Add }
/// FloatingActionButton::new(Icons::Add).on_press(Msg::Add);
/// FloatingActionButton::extended("New list").icon(Icons::Add).on_press(Msg::Add);
/// ```
///
/// This framework had no such widget. It had `fab_button`, a **helper returning a
/// [`Button`](crate::Button)** — which meant the scaffold's floating action button was a
/// filled button in disguise, and took a filled button's colours: `primary` on
/// `on_primary` where the reference's takes `primary_container` on `on_primary_container`
/// (`floating_action_button.dart:809`). Two roles out, on the most prominent control on
/// the screen.
pub struct FloatingActionButton<Msg> {
    icon: Option<Icons>,
    label: Option<String>,
    size: FabSize,
    enabled: bool,
    on_press: Option<Msg>,
    background: Option<Color>,
    foreground: Option<Color>,
    elevation: Option<f32>,
    shape: Option<ShapeBorder>,
    icon_size: Option<f32>,
    label_style: Option<TextStyle>,
}

impl<Msg: Clone> FloatingActionButton<Msg> {
    /// A button carrying a glyph.
    pub fn new(icon: Icons) -> Self {
        Self {
            icon: Some(icon),
            label: None,
            size: FabSize::Regular,
            enabled: true,
            on_press: None,
            background: None,
            foreground: None,
            elevation: None,
            shape: None,
            icon_size: None,
            label_style: None,
        }
    }

    /// A button carrying **one character** rather than a drawn icon — a plus, a tick, an
    /// arrow. The same escape hatch [`IconButton::glyph`](crate::IconButton::glyph)
    /// offers, and for the same reason: not every mark an application wants is in
    /// [`Icons`], and waiting for one to be added is not an answer.
    ///
    /// It is a **regular** button, not an extended one: the character is centred in the
    /// square the way a glyph would be, and the box does not grow to fit it.
    pub fn glyph(text: impl Into<String>) -> Self {
        Self {
            label: Some(text.into()),
            ..Self::extended("")
        }
        .size(FabSize::Regular)
    }

    /// An **extended** button: words, and optionally a glyph before them through
    /// [`icon`](Self::icon). For an action whose glyph would not say what it does.
    pub fn extended(label: impl Into<String>) -> Self {
        Self {
            icon: None,
            label: Some(label.into()),
            size: FabSize::Extended,
            enabled: true,
            on_press: None,
            background: None,
            foreground: None,
            elevation: None,
            shape: None,
            icon_size: None,
            label_style: None,
        }
    }

    /// The glyph, over whatever was there. On an extended button it goes before the
    /// words.
    #[must_use]
    pub fn icon(mut self, icon: Icons) -> Self {
        self.icon = Some(icon);
        self
    }

    /// **The smaller box**: 40 across, cornered at 12, with the same glyph.
    #[must_use]
    pub fn small(mut self) -> Self {
        self.size = FabSize::Small;
        self
    }

    /// **The larger box**: 96 across, cornered at 28, with a 36 glyph.
    #[must_use]
    pub fn large(mut self) -> Self {
        self.size = FabSize::Large;
        self
    }

    /// Which of the four this is, said outright.
    #[must_use]
    pub fn size(mut self, size: FabSize) -> Self {
        self.size = size;
        self
    }

    /// What it does. A button with no message is **inert**, which is not the same as
    /// disabled: see [`enabled`](Self::enabled).
    #[must_use]
    pub fn on_press(mut self, message: Msg) -> Self {
        self.on_press = Some(message);
        self
    }

    /// Whether it can be pressed. Disabled, it flattens to `on_surface` at 12 % under a
    /// glyph at 38 % and **casts no shadow** — the reference's `disabledElevation` is
    /// zero, and a thing that is still floating while refusing to be pressed is telling
    /// two different stories.
    #[must_use]
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// **The surface**, over the theme's and `primary_container`.
    #[must_use]
    pub fn background(mut self, color: Color) -> Self {
        self.background = Some(color);
        self
    }

    /// **What is drawn on it**, over the theme's and `on_primary_container`.
    #[must_use]
    pub fn foreground(mut self, color: Color) -> Self {
        self.foreground = Some(color);
        self
    }

    /// How far off the page it sits at rest, in pixels. Six by default.
    #[must_use]
    pub fn elevation(mut self, elevation: f32) -> Self {
        self.elevation = Some(elevation);
        self
    }

    /// **What shape it is**, over the theme's and the corner its size asks for.
    #[must_use]
    pub fn shape(mut self, shape: ShapeBorder) -> Self {
        self.shape = Some(shape);
        self
    }

    /// The shorthand for a rounded rectangle. `shape(ShapeBorder::stadium())` is the
    /// round button a docked one needs — see the note on [`Self::shape_of`].
    #[must_use]
    pub fn radius(self, radius: impl Into<BorderRadius>) -> Self {
        self.shape(ShapeBorder::rounded(radius.into()))
    }

    /// How big the glyph is, over the theme's and the size's own.
    #[must_use]
    pub fn icon_size(mut self, size: f32) -> Self {
        self.icon_size = Some(size);
        self
    }

    /// An extended button's type, over the theme's and `label_large`
    /// (`floating_action_button.dart:832`).
    #[must_use]
    pub fn label_style(mut self, style: TextStyle) -> Self {
        self.label_style = Some(style);
        self
    }

    /// **What shape it is**: the caller's word, then the theme's, then the corner its
    /// size asks for.
    ///
    /// The framework's own is the reference's per-size rounding and **not** a circle.
    /// A docked button, sitting in the circular notch a
    /// [`BottomAppBar`](crate::BottomAppBar) cuts for it, wants
    /// [`ShapeBorder::stadium`] instead — the notch is round, and a rounded square in it
    /// leaves four corners hanging over the bar. `Scaffold` cannot pick for the caller,
    /// because it cuts the notch from the button's *bounds* and never sees its shape.
    /// See the roadmap.
    pub fn shape_of(&self, theme: &Theme) -> ShapeBorder {
        crate::resolve_shape(
            self.shape,
            theme.widgets.fab.shape,
            theme.widgets.fab.radius.map(BorderRadius::uniform),
            ShapeBorder::rounded(self.size.radius()),
        )
    }

    fn glyph_size(&self, theme: &Theme) -> f32 {
        self.icon_size
            .or(theme.widgets.fab.icon_size)
            .unwrap_or(self.size.icon_size())
    }

    fn text_style(&self, theme: &Theme) -> frus_core::ResolvedTextStyle {
        self.label_style
            .or(theme.widgets.fab.label_style)
            .unwrap_or(theme.text.label_large)
            .resolved()
    }

    /// The room before the content: sixteen when a glyph leads the words, twenty when
    /// the words stand alone (`floating_action_button.dart:831`).
    fn lead_pad(&self, theme: &Theme) -> f32 {
        theme.widgets.fab.extended_padding.unwrap_or(
            if self.icon.is_some() && self.size == FabSize::Extended {
                EXTENDED_PAD_WITH_ICON
            } else {
                EXTENDED_PAD
            },
        )
    }

    /// How wide it comes out: its own side, unless it is extended, where it is as wide as
    /// what it carries.
    fn width(&self, theme: &Theme) -> f32 {
        if self.size != FabSize::Extended {
            return self.size.extent();
        }
        let lead = self.lead_pad(theme);
        let glyph = match self.icon {
            Some(_) => {
                self.glyph_size(theme) + theme.widgets.fab.extended_gap.unwrap_or(EXTENDED_GAP)
            }
            None => 0.0,
        };
        let words = match &self.label {
            Some(label) => frus_text::measure_resolved(label, &self.text_style(theme)).width,
            None => 0.0,
        };
        lead + glyph + words + EXTENDED_PAD
    }

    /// The elevation this frame: none while disabled, the hovered value under a pointer,
    /// the resting one otherwise.
    fn depth(&self, status: &Status, theme: &Theme) -> f32 {
        if !self.enabled || self.on_press.is_none() {
            return 0.0;
        }
        let rest = self
            .elevation
            .or(theme.widgets.fab.elevation)
            .unwrap_or(ELEVATION);
        let hovered = theme
            .widgets
            .fab
            .hover_elevation
            .unwrap_or(HOVER_ELEVATION.max(rest));
        rest + (hovered - rest) * status.hover_progress.clamp(0.0, 1.0)
    }
}

impl<Msg: Clone> Widget<Msg> for FloatingActionButton<Msg> {
    fn style(&self) -> Style {
        Widget::<Msg>::style_themed(self, &Theme::default())
    }

    fn style_themed(&self, theme: &Theme) -> Style {
        Style {
            width: Dimension::Length(self.width(theme)),
            height: Dimension::Length(self.size.extent()),
            ..Default::default()
        }
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &[]
    }

    fn paint(&self, bounds: Rect, status: Status, theme: &Theme, scene: &mut Scene) {
        let o = status.opacity;
        let shape = self.shape_of(theme);
        let radius = shape
            .as_rounded(bounds)
            .map(|(_, r)| r)
            .unwrap_or(BorderRadius::ZERO);

        let (fill, ink) = if self.enabled {
            (
                self.background
                    .or(theme.widgets.fab.background)
                    .unwrap_or(theme.scheme.primary_container),
                self.foreground
                    .or(theme.widgets.fab.foreground)
                    .unwrap_or(theme.scheme.on_primary_container),
            )
        } else {
            // The same flattening `Button` and `Chip` do, for the same reason: the
            // reference collapses every variant to `on_surface` at 12 % under content at
            // 38 %, so unavailable reads as unavailable and not as a quieter button.
            (disabled_container(theme), disabled_content(theme))
        };

        let depth = self.depth(&status, theme);
        if depth > 0.0 {
            let blur = depth * 4.0 + 8.0;
            scene.shadow(
                Rect::new(
                    bounds.x - blur,
                    bounds.y + depth * 2.0 - blur,
                    bounds.width + 2.0 * blur,
                    bounds.height + 2.0 * blur,
                ),
                theme.scheme.shadow.with_alpha(0.30).fade(o),
                radius.inflate(blur),
                blur,
            );
        }
        // The state layer sits **on** the surface, in the content's colour, which is what
        // keeps a hovered button from looking like a differently-coloured one.
        let surface = theme.state_layer(fill, ink, &status);
        scene.draw_shape(bounds, shape, surface.fade(o));

        let glyph = self.glyph_size(theme);
        let cy = bounds.y + (bounds.height - glyph) * 0.5;
        match (&self.label, self.icon) {
            // Extended: the glyph, then the words, both against the leading edge.
            (Some(label), icon) if self.size == FabSize::Extended => {
                let mut x = bounds.x + self.lead_pad(theme);
                if let Some(name) = icon {
                    let path = name.path().scaled(glyph / ICON_GRID).translated(x, cy);
                    scene.fill_path(&path, ink.fade(o));
                    x += glyph + theme.widgets.fab.extended_gap.unwrap_or(EXTENDED_GAP);
                }
                let style = self.text_style(theme);
                let measured = frus_text::measure_resolved(label, &style);
                scene.text(
                    Point::new(x, bounds.y + (bounds.height - measured.height) * 0.5),
                    label.clone(),
                    &style,
                    ink.fade(o),
                );
            }
            // Otherwise: whatever it carries, centred in the square. An icon wins over a
            // character, since a caller who gave both meant the drawn one.
            (_, Some(name)) => {
                let path = name
                    .path()
                    .scaled(glyph / ICON_GRID)
                    .translated(bounds.x + (bounds.width - glyph) * 0.5, cy);
                scene.fill_path(&path, ink.fade(o));
            }
            (Some(text), None) => {
                let style = self.text_style(theme);
                let measured = frus_text::measure_resolved(text, &style);
                scene.text(
                    Point::new(
                        bounds.x + (bounds.width - measured.width) * 0.5,
                        bounds.y + (bounds.height - measured.height) * 0.5,
                    ),
                    text.clone(),
                    &style,
                    ink.fade(o),
                );
            }
            (None, None) => {}
        }
    }

    fn on_click(&self) -> Option<Msg> {
        self.enabled.then(|| self.on_press.clone()).flatten()
    }

    fn ink(&self, theme: &Theme) -> Option<crate::InkStyle> {
        // Only where there is something to press. Ink on a disabled or inert button
        // promises an action it does not have.
        if !self.enabled {
            return None;
        }
        self.on_press.as_ref()?;
        let radius = self
            .shape_of(theme)
            .as_rounded(Rect::new(0.0, 0.0, self.width(theme), self.size.extent()))
            .map(|(_, r)| r)
            .unwrap_or(BorderRadius::ZERO);
        Some(crate::InkStyle::of(theme).radius(radius))
    }

    fn focusable(&self) -> bool {
        self.enabled && self.on_press.is_some()
    }

    /// It says what it does. An extended button has words to read out; a plain one has
    /// only a glyph, and a glyph is not a name — so the label is empty and the caller has
    /// to wrap it in a [`Tooltip`](crate::Tooltip), which is what the reference's own
    /// `tooltip` argument is for.
    fn semantics(&self) -> Option<frus_core::SemanticsProperties> {
        let semantics = frus_core::SemanticsProperties::new(frus_core::Role::Button);
        let semantics = match &self.label {
            Some(label) => semantics.label(label.clone()),
            None => semantics,
        };
        Some(if self.enabled && self.on_press.is_some() {
            semantics.clickable()
        } else {
            semantics.disabled(!self.enabled)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use frus_core::Primitive;

    #[derive(Clone, Debug, PartialEq)]
    enum Msg {
        Add,
    }

    fn painted(
        fab: &FloatingActionButton<Msg>,
        theme: &Theme,
        status: Status,
    ) -> (Vec<(Rect, Color, BorderRadius)>, usize) {
        let mut scene = Scene::new();
        let style = Widget::<Msg>::style_themed(fab, theme);
        let (Dimension::Length(w), Dimension::Length(h)) = (style.width, style.height) else {
            panic!("a measured box");
        };
        Widget::<Msg>::paint(fab, Rect::new(0.0, 0.0, w, h), status, theme, &mut scene);
        let rects = scene
            .primitives()
            .iter()
            .filter_map(|p| match p {
                Primitive::Rect {
                    rect,
                    color,
                    radius,
                    ..
                } => Some((*rect, *color, *radius)),
                _ => None,
            })
            .collect();
        let paths = scene
            .primitives()
            .iter()
            .filter(|p| matches!(p, Primitive::Path { .. }))
            .count();
        (rects, paths)
    }

    /// **The colours were two roles out.** `fab_button` returned a filled
    /// [`Button`](crate::Button), so the scaffold's floating action button took `primary`
    /// on `on_primary` where the reference's takes `primary_container` on
    /// `on_primary_container` (`floating_action_button.dart:809`) — on the most prominent
    /// control on the screen.
    #[test]
    fn a_button_takes_the_container_roles_and_not_the_accent() {
        let theme = Theme::default();
        let fab = FloatingActionButton::new(Icons::Add).on_press(Msg::Add);
        let (rects, paths) = painted(&fab, &theme, Status::default());
        assert_eq!(
            rects.last().expect("a surface").1,
            theme.scheme.primary_container,
            "the container, not the accent"
        );
        assert_ne!(
            theme.scheme.primary_container, theme.primary,
            "and they differ"
        );
        assert_eq!(paths, 1, "the glyph is drawn");
    }

    /// **The four sizes are one decision, not three.** Every one of the box, the corner
    /// and the glyph follows from *which* of the four it is
    /// (`floating_action_button.dart:783`, `:816`, `:824`), so they travel together and a
    /// caller cannot build a large button with a small one's corner.
    #[test]
    fn the_four_sizes_carry_their_own_box_corner_and_glyph() {
        let theme = Theme::default();
        for (size, extent, radius, icon) in [
            (FabSize::Small, 40.0, 12.0, 24.0),
            (FabSize::Regular, 56.0, 16.0, 24.0),
            (FabSize::Large, 96.0, 28.0, 36.0),
        ] {
            assert_eq!(size.extent(), extent);
            assert_eq!(size.radius(), radius);
            assert_eq!(size.icon_size(), icon);

            let fab = FloatingActionButton::<Msg>::new(Icons::Add).size(size);
            let style = Widget::<Msg>::style_themed(&fab, &theme);
            assert_eq!(style.width, Dimension::Length(extent));
            assert_eq!(style.height, Dimension::Length(extent));
            let (rects, _) = painted(&fab, &theme, Status::default());
            assert_eq!(
                rects.last().expect("a surface").2,
                BorderRadius::uniform(radius),
                "{size:?} takes its own corner"
            );
        }
    }

    /// **It is not a circle**, which is what `fab_button` drew. The reference rounds a
    /// regular button by sixteen and a small one by twelve
    /// (`floating_action_button.dart:816`); a stadium takes half the short side, so both
    /// were pills. A caller who *wants* the pill — a docked button, in a round notch —
    /// says so.
    #[test]
    fn it_is_a_rounded_square_and_a_stadium_when_asked() {
        let theme = Theme::default();
        let plain = FloatingActionButton::<Msg>::new(Icons::Add);
        assert_eq!(
            painted(&plain, &theme, Status::default())
                .0
                .last()
                .unwrap()
                .2,
            BorderRadius::uniform(16.0)
        );
        let docked = FloatingActionButton::<Msg>::new(Icons::Add).shape(ShapeBorder::stadium());
        assert_eq!(
            painted(&docked, &theme, Status::default())
                .0
                .last()
                .unwrap()
                .2,
            BorderRadius::uniform(28.0),
            "half the short side"
        );
    }

    /// **An extended button is as wide as its words**, with the reference's asymmetric
    /// room: sixteen before a glyph, twenty before words standing alone, twenty after
    /// either (`floating_action_button.dart:831`).
    #[test]
    fn an_extended_button_is_as_wide_as_what_it_carries() {
        let theme = Theme::default();
        let words = FloatingActionButton::<Msg>::extended("New list");
        let with_icon = FloatingActionButton::<Msg>::extended("New list").icon(Icons::Add);

        let width = |fab: &FloatingActionButton<Msg>| match Widget::<Msg>::style_themed(fab, &theme)
            .width
        {
            Dimension::Length(w) => w,
            other => panic!("a measured width, not {other:?}"),
        };
        let text =
            frus_text::measure_resolved("New list", &theme.text.label_large.resolved()).width;
        assert_eq!(width(&words), EXTENDED_PAD + text + EXTENDED_PAD);
        assert_eq!(
            width(&with_icon),
            EXTENDED_PAD_WITH_ICON + 24.0 + EXTENDED_GAP + text + EXTENDED_PAD,
            "a glyph shortens the leading room, as the reference's does"
        );
        assert_eq!(
            Widget::<Msg>::style_themed(&words, &theme).height,
            Dimension::Length(56.0),
            "and it is still a regular button's height"
        );

        let (_, paths) = painted(&with_icon, &theme, Status::default());
        assert_eq!(paths, 1, "the glyph before the words");
        assert_eq!(painted(&words, &theme, Status::default()).1, 0);
    }

    /// **A disabled button casts no shadow.** The reference's `disabledElevation` is
    /// zero, and a control still floating while refusing to be pressed is telling two
    /// different stories at once. It also stops answering, stops taking focus and stops
    /// splashing.
    #[test]
    fn a_disabled_button_stops_floating() {
        let theme = Theme::default();
        let live = FloatingActionButton::new(Icons::Add).on_press(Msg::Add);
        let (rects, _) = painted(&live, &theme, Status::default());
        assert_eq!(rects.len(), 2, "a shadow under the surface: {rects:#?}");

        let dead = FloatingActionButton::new(Icons::Add)
            .on_press(Msg::Add)
            .enabled(false);
        let (rects, _) = painted(&dead, &theme, Status::default());
        assert_eq!(rects.len(), 1, "the surface alone");
        assert_eq!(rects[0].1, disabled_container(&theme));
        assert_eq!(Widget::<Msg>::on_click(&dead), None);
        assert!(!Widget::<Msg>::focusable(&dead));
        assert!(Widget::<Msg>::ink(&dead, &theme).is_none());

        // And one with nothing to say does not float either: it is inert, not disabled,
        // so it keeps its live colours and loses its shadow.
        let inert = FloatingActionButton::<Msg>::new(Icons::Add);
        let (rects, _) = painted(&inert, &theme, Status::default());
        assert_eq!(rects.len(), 1);
        assert_eq!(rects[0].1, theme.scheme.primary_container);
    }

    /// **It rises under a pointer**, six to eight (`floating_action_button.dart:778`),
    /// which is the whole of what a floating button's hover state is.
    #[test]
    fn it_rises_under_a_pointer() {
        let theme = Theme::default();
        let fab = FloatingActionButton::new(Icons::Add).on_press(Msg::Add);
        let shadow_of = |hover: f32| {
            let status = Status {
                hover_progress: hover,
                ..Default::default()
            };
            painted(&fab, &theme, status).0[0].0.width
        };
        assert!(
            shadow_of(1.0) > shadow_of(0.0),
            "a wider shadow is a higher button"
        );
    }

    /// Every part of it answers to a theme, and to a caller over the theme.
    #[test]
    fn a_theme_answers_for_every_button() {
        let mut theme = Theme::default();
        theme.widgets.fab.background = Some(Color::rgb(0.2, 0.4, 0.6));
        theme.widgets.fab.foreground = Some(Color::rgb(0.9, 0.9, 0.9));
        theme.widgets.fab.radius = Some(3.0);
        theme.widgets.fab.icon_size = Some(11.0);

        let fab = FloatingActionButton::new(Icons::Add).on_press(Msg::Add);
        let (rects, _) = painted(&fab, &theme, Status::default());
        let surface = rects.last().unwrap();
        assert_eq!(surface.1, Color::rgb(0.2, 0.4, 0.6));
        assert_eq!(surface.2, BorderRadius::uniform(3.0));

        let told = FloatingActionButton::new(Icons::Add)
            .on_press(Msg::Add)
            .background(Color::rgb(0.1, 0.1, 0.1))
            .radius(9.0);
        let (rects, _) = painted(&told, &theme, Status::default());
        let surface = rects.last().unwrap();
        assert_eq!(surface.1, Color::rgb(0.1, 0.1, 0.1));
        assert_eq!(surface.2, BorderRadius::uniform(9.0));
    }

    /// **A character is not an extended button.** `fab_button` hands the scaffold a
    /// plus, a tick or an arrow — one mark, in a square, centred like a glyph — and the
    /// widget has to be able to say that without growing to fit the words it does not
    /// have. `IconButton::glyph` is the same escape hatch for the same reason.
    #[test]
    fn a_character_sits_in_the_square_like_a_glyph() {
        let theme = Theme::default();
        let plus = FloatingActionButton::<Msg>::glyph("+");
        let style = Widget::<Msg>::style_themed(&plus, &theme);
        assert_eq!(
            style.width,
            Dimension::Length(56.0),
            "a square, not a lozenge"
        );
        assert_eq!(style.height, Dimension::Length(56.0));

        // And the scaffold's shorthand is that, rounded to fit the notch a bottom bar
        // cuts for it — which is the one place the reference's own corner is wrong here.
        let docked = crate::fab_button::<Msg>("+", Msg::Add);
        let (rects, _) = painted(&docked, &theme, Status::default());
        assert_eq!(
            rects.last().unwrap().2,
            BorderRadius::uniform(28.0),
            "round, for a round notch"
        );
        assert_eq!(
            rects.last().unwrap().1,
            theme.scheme.primary_container,
            "and it takes the button's colours now, not a filled button's"
        );
    }

    /// **A glyph is not a name.** An extended button reads out its words; a plain one has
    /// nothing to read, so it says so by having no label rather than by inventing one —
    /// and a caller who wants it spoken wraps it in a
    /// [`Tooltip`](crate::Tooltip), which is what the reference's own `tooltip` argument
    /// is for.
    #[test]
    fn an_extended_button_has_something_to_read_out() {
        let words = FloatingActionButton::extended("New list").on_press(Msg::Add);
        let semantics = Widget::<Msg>::semantics(&words).expect("a button");
        assert_eq!(semantics.label.as_deref(), Some("New list"));
        assert!(semantics.clickable);

        let glyph = FloatingActionButton::new(Icons::Add).on_press(Msg::Add);
        assert_eq!(Widget::<Msg>::semantics(&glyph).unwrap().label, None);
    }
}
