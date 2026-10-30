//! A desktop window standing in for a phone or a tablet (milestone 721).
//!
//! With `FRUS_DEVICE` naming a [`DeviceProfile`] in a debug build, the application is laid
//! out for that device's screen — its logical size, its safe areas, its platform — and the
//! window shows it inside the device's outline: the body and its buttons, the screen's
//! rounded corners, the cutout, a status bar and a home indicator painted by the shell.
//!
//! The window is the device **zoomed** to fit: a tablet on a small monitor is shown smaller,
//! never cropped. The application is told nothing of the zoom; it draws at the device's
//! size, and the shell maps the pointer back into the screen.
//!
//! Everything here is painted by frus, so it is the same on every development system.

use frus_widgets::{
    BorderRadius, Brightness, ClipShape, Color, DeviceProfile, FontWeight, LayerFilter,
    Orientation, Point, Primitive, Rect, Scene, Size, TextStyle, WindowInsets,
};

/// The room left around the device in the window, in logical pixels.
const MARGIN: f32 = 24.0;

/// The variable that names the device on a desktop, as `frus run -d` sets it.
pub(crate) const VARIABLE: &str = "FRUS_DEVICE";
/// What names it on a page: a property of `window`, which `frus run -d <device> --web` sets
/// before the application loads (milestone 724). Not the address, which is the router's.
#[cfg_attr(not(web), allow(dead_code))]
const PAGE_PROPERTY: &str = "__frusDevice";

/// The window's background around the device.
const BACKDROP: Color = Color::rgb(0.13, 0.13, 0.14);
/// The device's body.
const BODY: Color = Color::rgb(0.08, 0.08, 0.09);
/// The edge of the body, a shade lighter so it reads against the backdrop.
const BODY_EDGE: Color = Color::rgb(0.30, 0.30, 0.32);

/// The device a window stands in for, and how it is held.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Simulated {
    pub(crate) profile: DeviceProfile,
    pub(crate) orientation: Orientation,
    /// The keyboard is up because a field that takes typing has the focus.
    pub(crate) keyboard: bool,
    /// The keyboard is held up by the developer (`K`), focus or not.
    pub(crate) keyboard_held: bool,
}

/// Where the device sits in the window, in the window's logical pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Placement {
    /// Window pixels per device pixel; never more than 1.
    pub(crate) zoom: f32,
    /// The device's body.
    pub(crate) body: Rect,
    /// The screen.
    pub(crate) screen: Rect,
}

impl Simulated {
    /// The device asked for, if one is: `FRUS_DEVICE` on a desktop, `window.__frusDevice` on
    /// a page.
    ///
    /// Debug builds only: a released application is never a simulation. An unknown name
    /// says which ones are known, and the window opens as itself.
    pub(crate) fn from_env() -> Option<Self> {
        let wanted = asked()?;
        let wanted = wanted.trim();
        if wanted.is_empty() {
            return None;
        }
        if !cfg!(debug_assertions) {
            log::warn!("{VARIABLE} is ignored in a release build");
            return None;
        }
        match Self::named(wanted) {
            Ok(simulated) => {
                log::info!(
                    "simulating {} ({}, {}×{} at {}x)",
                    simulated.profile.name,
                    simulated.profile.platform,
                    simulated.profile.screen.width,
                    simulated.profile.screen.height,
                    simulated.profile.pixel_ratio
                );
                Some(simulated)
            }
            Err(err) => {
                log::warn!("{err}");
                None
            }
        }
    }

    /// The built-in device called `wanted`, by id or name, held upright.
    pub(crate) fn named(wanted: &str) -> Result<Self, String> {
        let profiles = DeviceProfile::builtin();
        match DeviceProfile::find(&profiles, wanted) {
            Some(profile) => Ok(Simulated {
                profile: profile.clone(),
                orientation: Orientation::Portrait,
                keyboard: false,
                keyboard_held: false,
            }),
            None => {
                let known: Vec<&str> = profiles.iter().map(|p| p.id.as_str()).collect();
                Err(format!(
                    "simulated device `{wanted}`: no such device; known: {}",
                    known.join(", ")
                ))
            }
        }
    }

    /// Whether the soft keyboard is up: asked for by a field, or held up.
    pub(crate) fn keyboard_up(&self) -> bool {
        self.keyboard || self.keyboard_held
    }

    /// The screen the application lays out for, in the device's logical pixels.
    pub(crate) fn screen_size(&self) -> Size {
        self.profile.screen_size(self.orientation)
    }

    /// The insets the application is told about.
    pub(crate) fn insets(&self) -> WindowInsets {
        self.profile
            .window_insets(self.orientation, self.keyboard_up())
    }

    /// The window that shows the device at its own size, margin included.
    pub(crate) fn window_size(&self) -> Size {
        let outer = self.profile.outer_size(self.orientation);
        Size::new(outer.width + 2.0 * MARGIN, outer.height + 2.0 * MARGIN)
    }

    /// Where the device goes in a window of `window` logical pixels: as large as fits, no
    /// larger than itself, centred.
    pub(crate) fn place(&self, window: Size) -> Placement {
        let outer = self.profile.outer_size(self.orientation);
        let room = self.window_size();
        let zoom = (window.width / room.width)
            .min(window.height / room.height)
            .clamp(0.05, 1.0);
        let (w, h) = (outer.width * zoom, outer.height * zoom);
        let body = Rect::new((window.width - w) / 2.0, (window.height - h) / 2.0, w, h);
        let s = self.profile.screen_rect(self.orientation);
        let screen = Rect::new(
            body.x + s.x * zoom,
            body.y + s.y * zoom,
            s.width * zoom,
            s.height * zoom,
        );
        Placement { zoom, body, screen }
    }

    /// The point of the application's screen under `point`, in the window's logical
    /// pixels.
    pub(crate) fn to_screen(placement: Placement, point: Point) -> Point {
        Point::new(
            (point.x - placement.screen.x) / placement.zoom,
            (point.y - placement.screen.y) / placement.zoom,
        )
    }

    /// Where `rect`, in the application's pixels, is in the window: the reverse of
    /// [`to_screen`](Self::to_screen), for what the shell puts over the canvas.
    #[cfg_attr(not(web), allow(dead_code))]
    pub(crate) fn to_window(placement: Placement, rect: Rect) -> Rect {
        Rect::new(
            placement.screen.x + rect.x * placement.zoom,
            placement.screen.y + rect.y * placement.zoom,
            rect.width * placement.zoom,
            rect.height * placement.zoom,
        )
    }

    /// The window's frame: the device around `app`, the application's scene already in
    /// physical pixels at `scale × zoom` with its origin at the screen's corner.
    ///
    /// `scale` is the window's physical pixels per logical one; `window` its logical size;
    /// `icons` whether the status bar's icons are dark or light, as the application asked;
    /// `system` the device's own brightness, which the keyboard follows.
    pub(crate) fn paint(
        &self,
        app: &Scene,
        placement: Placement,
        scale: f32,
        window: Size,
        icons: Brightness,
        system: Brightness,
    ) -> Scene {
        let px = |r: Rect| Rect::new(r.x * scale, r.y * scale, r.width * scale, r.height * scale);
        let device = scale * placement.zoom;
        let profile = &self.profile;
        let landscape = self.orientation == Orientation::Landscape;
        let mut scene = Scene::new();

        scene.fill_rect(
            px(Rect::new(0.0, 0.0, window.width, window.height)),
            BACKDROP,
        );
        let body = px(placement.body);
        let body_radius = profile.frame.radius * device;
        scene.shadow(
            body,
            Color::rgba(0.0, 0.0, 0.0, 0.5),
            body_radius,
            24.0 * scale,
        );

        // The buttons stand out of the body; the body is painted over their inner half.
        for button in &profile.frame.buttons {
            let depth = 3.0 * device;
            let along = button.top * device;
            let length = button.length * device;
            let left = button.edge == frus_widgets::ButtonEdge::Left;
            let rect = if landscape {
                // Turned counter-clockwise: the left edge is at the bottom, the right one at
                // the top, and "down the edge" runs left to right.
                let x = body.x + along;
                let y = if left {
                    body.y + body.height - depth
                } else {
                    body.y - depth
                };
                Rect::new(x, y, length, 2.0 * depth)
            } else {
                let x = if left {
                    body.x - depth
                } else {
                    body.x + body.width - depth
                };
                Rect::new(x, body.y + along, 2.0 * depth, length)
            };
            scene.draw_rect(rect, BODY_EDGE, depth, 0.0, Color::TRANSPARENT);
        }
        scene.draw_rect(body, BODY, body_radius, 1.5 * scale, BODY_EDGE);

        let screen = px(placement.screen);
        let screen_radius = profile.screen_radius * device;
        // The glass under the application, as round as the screen: square, its corners
        // would show past the body's.
        scene.draw_rect(screen, Color::BLACK, screen_radius, 0.0, Color::TRANSPARENT);
        let translated: Vec<Primitive> = app
            .primitives()
            .iter()
            .map(|p| p.translated(screen.x, screen.y))
            .collect();
        scene.push_primitive(Primitive::Layer {
            primitives: translated,
            opacity: 1.0,
            clip: screen,
            clip_shape: ClipShape::RRect(BorderRadius::uniform(screen_radius)),
            transform: None,
            filter: LayerFilter::NONE,
            owner: 0,
        });

        // What the system paints over the application, in a layer of its own so that its
        // text is not drawn under the application's shapes.
        scene.layer(1.0, |over| {
            let ink = match icons {
                Brightness::Dark => Color::BLACK,
                Brightness::Light => Color::WHITE,
            };
            let insets = profile.insets_in(self.orientation).safe;
            // iOS hides its status bar on its side; Android keeps it.
            let status_bar = !landscape || profile.platform != frus_widgets::TargetPlatform::Ios;
            if status_bar && insets.top > 0.0 {
                paint_status_bar(over, screen, insets.top * device, device, ink, profile);
            }
            if let Some(cut) = profile.cutout_rect(self.orientation) {
                let cut = Rect::new(
                    screen.x + cut.x * device,
                    screen.y + cut.y * device,
                    cut.width * device,
                    cut.height * device,
                );
                let radius = match profile.cutout {
                    frus_widgets::Cutout::Notch { radius, .. } => radius * device,
                    _ => cut.width.min(cut.height) / 2.0,
                };
                over.draw_rect(cut, Color::BLACK, radius, 0.0, Color::TRANSPARENT);
            }
            let mut ink = ink;
            if self.keyboard_up() {
                let height = profile.insets_in(self.orientation).keyboard * device;
                let keyboard = Rect::new(
                    screen.x,
                    screen.y + screen.height - height,
                    screen.width,
                    height,
                );
                paint_keyboard(over, keyboard, device, system);
                // The indicator stands on the keyboard now, so it reads against that.
                ink = match system {
                    Brightness::Light => Color::BLACK,
                    Brightness::Dark => Color::WHITE,
                };
            }
            if !profile.frame.home_button && insets.bottom > 0.0 {
                let (w, h) = (134.0 * device, 5.0 * device);
                let indicator = Rect::new(
                    screen.x + (screen.width - w) / 2.0,
                    screen.y + screen.height - 8.0 * device - h,
                    w,
                    h,
                );
                over.draw_rect(indicator, ink, h / 2.0, 0.0, Color::TRANSPARENT);
            }
        });

        if profile.frame.home_button {
            let d = 0.7 * profile.frame.bezel.bottom * device;
            let centre = if landscape {
                Point::new(
                    body.x + body.width - profile.frame.bezel.bottom * device / 2.0,
                    body.y + body.height / 2.0,
                )
            } else {
                Point::new(
                    body.x + body.width / 2.0,
                    body.y + body.height - profile.frame.bezel.bottom * device / 2.0,
                )
            };
            let button = Rect::new(centre.x - d / 2.0, centre.y - d / 2.0, d, d);
            scene.draw_rect(button, BODY, d / 2.0, 2.0 * scale, BODY_EDGE);
        }
        scene
    }
}

/// A soft keyboard in `area`: three rows of letter keys and a row with the space bar, under
/// any suggestions bar's room. A picture only: typing comes from the desktop's keyboard.
fn paint_keyboard(scene: &mut Scene, area: Rect, device: f32, system: Brightness) {
    let (panel, key, wide, label) = match system {
        Brightness::Light => (
            Color::rgb(0.82, 0.83, 0.85),
            Color::WHITE,
            Color::rgb(0.67, 0.69, 0.73),
            Color::BLACK,
        ),
        Brightness::Dark => (
            Color::rgb(0.17, 0.17, 0.18),
            Color::rgb(0.42, 0.42, 0.43),
            Color::rgb(0.27, 0.27, 0.28),
            Color::WHITE,
        ),
    };
    scene.fill_rect(area, panel);
    let gap = 6.0 * device;
    // Four rows of keys share what is left under a band kept for suggestions.
    let top = area.y + 44.0 * device;
    let rows_height = (area.height - 44.0 * device - 2.0 * gap).max(0.0);
    let row_height = (rows_height / 4.0).min(54.0 * device);
    let key_height = row_height - gap;
    let unit = (area.width - gap) / 10.0;
    let style = TextStyle::default().size(20.0 * device).resolved();
    let rows: [&str; 3] = ["qwertyuiop", "asdfghjkl", "zxcvbnm"];
    for (r, letters) in rows.iter().enumerate() {
        let count = letters.chars().count() as f32;
        let start = area.x + gap / 2.0 + (10.0 - count) * unit / 2.0;
        let y = top + r as f32 * row_height;
        for (i, letter) in letters.chars().enumerate() {
            let x = start + i as f32 * unit;
            let rect = Rect::new(x + gap / 2.0, y, unit - gap, key_height);
            scene.draw_rect(rect, key, 5.0 * device, 0.0, Color::TRANSPARENT);
            scene.text(
                Point::new(rect.x + rect.width * 0.3, rect.y + key_height * 0.18),
                letter.to_string(),
                &style,
                label,
            );
        }
        if r == 2 {
            // Shift and delete, either side of the last row.
            let side = (start - area.x - gap).max(unit);
            for x in [area.x + gap, area.x + area.width - gap - side] {
                let rect = Rect::new(x, y, side - gap / 2.0, key_height);
                scene.draw_rect(rect, wide, 5.0 * device, 0.0, Color::TRANSPARENT);
            }
        }
    }
    let y = top + 3.0 * row_height;
    let side = 2.5 * unit;
    let space = Rect::new(
        area.x + gap + side,
        y,
        area.width - 2.0 * (gap + side) - gap,
        key_height,
    );
    scene.draw_rect(space, key, 5.0 * device, 0.0, Color::TRANSPARENT);
    for x in [area.x + gap / 2.0, space.x + space.width + gap] {
        let rect = Rect::new(x + gap / 2.0, y, side - gap, key_height);
        scene.draw_rect(rect, wide, 5.0 * device, 0.0, Color::TRANSPARENT);
    }
}

/// The name the developer gave the device, if they gave one.
fn asked() -> Option<String> {
    #[cfg(web)]
    {
        let window = web_sys::window()?;
        js_sys::Reflect::get(&window, &PAGE_PROPERTY.into())
            .ok()?
            .as_string()
    }
    #[cfg(not(web))]
    {
        std::env::var(VARIABLE).ok()
    }
}

/// The status bar: the time on one side, the signal and the battery on the other, centred
/// in the band of `height` physical pixels at the top of `screen`.
fn paint_status_bar(
    scene: &mut Scene,
    screen: Rect,
    height: f32,
    device: f32,
    ink: Color,
    profile: &DeviceProfile,
) {
    // Clear of the screen's rounded corners.
    let side = (profile.screen_radius * 0.6).max(16.0) * device;
    let size = 15.0 * device;
    let middle = screen.y + height / 2.0;
    let style = TextStyle::default()
        .size(size)
        .weight(FontWeight::SemiBold)
        .resolved();
    scene.text(
        Point::new(screen.x + side, middle - size * 0.65),
        "12:30",
        &style,
        ink,
    );

    let right = screen.x + screen.width - side;
    let (bw, bh) = (24.0 * device, 11.5 * device);
    let battery = Rect::new(right - bw - 2.0 * device, middle - bh / 2.0, bw, bh);
    scene.draw_rect(battery, Color::TRANSPARENT, 3.0 * device, 1.0 * device, ink);
    let inset = 2.0 * device;
    scene.draw_rect(
        Rect::new(
            battery.x + inset,
            battery.y + inset,
            (bw - 2.0 * inset) * 0.8,
            bh - 2.0 * inset,
        ),
        ink,
        1.5 * device,
        0.0,
        Color::TRANSPARENT,
    );
    scene.draw_rect(
        Rect::new(
            right - 1.5 * device,
            middle - 2.0 * device,
            1.5 * device,
            4.0 * device,
        ),
        ink,
        0.75 * device,
        0.0,
        Color::TRANSPARENT,
    );

    let bar = 3.0 * device;
    let gap = 1.5 * device;
    let start = battery.x - 6.0 * device - 4.0 * bar - 3.0 * gap;
    for i in 0..4 {
        let h = (4.0 + 2.5 * i as f32) * device;
        let x = start + i as f32 * (bar + gap);
        scene.draw_rect(
            Rect::new(x, middle + 5.5 * device - h, bar, h),
            ink,
            1.0 * device,
            0.0,
            Color::TRANSPARENT,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn phone() -> Simulated {
        Simulated::named("iphone-15").unwrap()
    }

    /// **A device is found by its id or its name, and a typo says what exists.**
    #[test]
    fn a_device_is_named_by_id_or_name() {
        assert_eq!(Simulated::named("iPhone 15").unwrap(), phone());
        let err = Simulated::named("iphone-51").unwrap_err();
        assert!(err.contains("no such device"), "{err}");
        assert!(
            err.contains("iphone-15") && err.contains("android-phone"),
            "{err}"
        );
    }

    /// **The application is told the device's screen and insets, not the window's.**
    #[test]
    fn the_application_sees_the_device() {
        let mut sim = phone();
        assert_eq!(sim.screen_size(), Size::new(393.0, 852.0));
        assert_eq!(sim.insets().padding.top, 59.0);
        sim.keyboard = true;
        assert_eq!(sim.insets().view_insets.bottom, 336.0);
        sim.orientation = Orientation::Landscape;
        assert_eq!(sim.screen_size(), Size::new(852.0, 393.0));
    }

    /// **A window big enough shows the device at its own size, centred; a smaller one
    /// shrinks it to fit, never past the window.**
    #[test]
    fn the_device_fits_the_window() {
        let sim = phone();
        let room = sim.window_size();
        assert_eq!(room, Size::new(421.0 + 48.0, 880.0 + 48.0));

        let big = sim.place(Size::new(1000.0, 1000.0));
        assert_eq!(big.zoom, 1.0);
        assert_eq!(big.body, Rect::new(289.5, 60.0, 421.0, 880.0));
        assert_eq!(big.screen, Rect::new(303.5, 74.0, 393.0, 852.0));

        let small = sim.place(Size::new(room.width, room.height / 2.0));
        assert_eq!(small.zoom, 0.5);
        assert!(small.body.y >= 0.0 && small.body.y + small.body.height <= room.height / 2.0);
        assert_eq!(small.screen.width, 393.0 * 0.5);
    }

    /// **The pointer lands where it is on the screen**, in the device's pixels, whatever
    /// the zoom: the screen's corner is the application's origin.
    #[test]
    fn the_pointer_is_mapped_into_the_screen() {
        let sim = phone();
        let room = sim.window_size();
        for window in [
            Size::new(1000.0, 1000.0),
            Size::new(room.width / 2.0, room.height / 2.0),
        ] {
            let at = sim.place(window);
            let corner = Simulated::to_screen(at, Point::new(at.screen.x, at.screen.y));
            assert!(corner.x.abs() < 1e-3 && corner.y.abs() < 1e-3);
            let far = Simulated::to_screen(
                at,
                Point::new(
                    at.screen.x + at.screen.width,
                    at.screen.y + at.screen.height,
                ),
            );
            assert!((far.x - 393.0).abs() < 1e-3 && (far.y - 852.0).abs() < 1e-3);
        }
    }

    /// **What the shell puts over a field lands on the field**: a rectangle of the
    /// application's goes to the window and back unchanged, at any zoom (milestone 724).
    #[test]
    fn a_rect_goes_to_the_window_and_back() {
        let sim = phone();
        let room = sim.window_size();
        let field = Rect::new(20.0, 400.0, 300.0, 48.0);
        for window in [
            Size::new(1000.0, 1000.0),
            Size::new(room.width / 2.0, room.height / 2.0),
        ] {
            let at = sim.place(window);
            let there = Simulated::to_window(at, field);
            assert!((there.width - field.width * at.zoom).abs() < 1e-3);
            let corner = Simulated::to_screen(at, Point::new(there.x, there.y));
            let far = Simulated::to_screen(
                at,
                Point::new(there.x + there.width, there.y + there.height),
            );
            assert!((corner.x - 20.0).abs() < 1e-3 && (corner.y - 400.0).abs() < 1e-3);
            assert!((far.x - 320.0).abs() < 1e-3 && (far.y - 448.0).abs() < 1e-3);
        }
    }

    /// **The application's frame is inside the screen, clipped to its corners**, with the
    /// device painted around it and the island over it.
    #[test]
    fn the_application_is_drawn_in_the_screen() {
        let sim = phone();
        let window = Size::new(1000.0, 1000.0);
        let at = sim.place(window);
        let scale = 2.0;
        let mut app = Scene::new();
        app.fill_rect(
            Rect::new(0.0, 0.0, 393.0 * scale, 852.0 * scale),
            Color::WHITE,
        );
        let frame = sim.paint(&app, at, scale, window, Brightness::Dark, Brightness::Light);

        let layer = frame
            .primitives()
            .iter()
            .find_map(|p| match p {
                Primitive::Layer {
                    primitives,
                    clip,
                    clip_shape: ClipShape::RRect(radius),
                    ..
                } => Some((primitives, *clip, *radius)),
                _ => None,
            })
            .expect("the application's layer");
        let (inner, clip, radius) = layer;
        assert_eq!(
            clip,
            Rect::new(303.5 * scale, 74.0 * scale, 393.0 * scale, 852.0 * scale)
        );
        assert_eq!(radius, BorderRadius::uniform(55.0 * scale));
        // The glass under it is as round: square, its corners showed past the body's.
        assert!(frame.primitives().iter().any(|p| matches!(
            p,
            Primitive::Rect { rect, color, radius: r, .. }
                if *rect == clip && *color == Color::BLACK && *r == radius
        )));
        assert!(!frame.primitives().iter().any(|p| matches!(
            p,
            Primitive::Rect { rect, radius: r, .. } if *rect == clip && *r == BorderRadius::ZERO
        )));
        match &inner[0] {
            Primitive::Rect { rect, .. } => assert_eq!((rect.x, rect.y), (clip.x, clip.y)),
            other => panic!("{other:?}"),
        }

        // The island: black, over the screen, after the application.
        let island = Rect::new(
            (303.5 + 133.5) * scale,
            (74.0 + 11.0) * scale,
            126.0 * scale,
            37.0 * scale,
        );
        let over = frame
            .primitives()
            .iter()
            .rev()
            .find_map(|p| match p {
                Primitive::Layer {
                    primitives,
                    clip_shape: ClipShape::Rect,
                    ..
                } => Some(primitives),
                _ => None,
            })
            .expect("the system's layer");
        assert!(over.iter().any(|p| matches!(
            p,
            Primitive::Rect { rect, color, .. } if *rect == island && *color == Color::BLACK
        )));
        // The status bar's time, in dark ink as asked.
        assert!(over.iter().any(|p| matches!(
            p,
            Primitive::Text { text, color, .. } if text == "12:30" && *color == Color::BLACK
        )));
    }

    /// **On its side an iPhone has no status bar**, and the island moves to the left.
    #[test]
    fn landscape_hides_the_ios_status_bar() {
        let mut sim = phone();
        sim.orientation = Orientation::Landscape;
        let window = Size::new(1000.0, 1000.0);
        let at = sim.place(window);
        let frame = sim.paint(
            &Scene::new(),
            at,
            1.0,
            window,
            Brightness::Light,
            Brightness::Light,
        );
        let texts = frame
            .primitives()
            .iter()
            .filter_map(|p| match p {
                Primitive::Layer { primitives, .. } => Some(primitives),
                _ => None,
            })
            .flatten()
            .filter(|p| matches!(p, Primitive::Text { .. }))
            .count();
        assert_eq!(texts, 0);
        assert!(at.screen.width > at.screen.height);
    }

    /// The panel the keyboard is drawn on, if one is: the widest rectangle in the system's
    /// layer that touches the bottom of the screen.
    fn keyboard_panel(frame: &Scene, screen: Rect) -> Option<(Rect, Color)> {
        frame
            .primitives()
            .iter()
            .filter_map(|p| match p {
                Primitive::Layer {
                    primitives,
                    clip_shape: ClipShape::Rect,
                    ..
                } => Some(primitives),
                _ => None,
            })
            .flatten()
            .find_map(|p| match p {
                Primitive::Rect { rect, color, .. }
                    if (rect.width - screen.width).abs() < 1e-3
                        && (rect.y + rect.height - (screen.y + screen.height)).abs() < 1e-3 =>
                {
                    Some((*rect, *color))
                }
                _ => None,
            })
    }

    /// **The keyboard is drawn as high as the profile says, at the bottom of the screen,
    /// in the device's brightness** — whether a field asked for it or it is held up.
    #[test]
    fn the_keyboard_is_drawn_where_the_insets_say() {
        let mut sim = phone();
        let window = Size::new(1000.0, 1000.0);
        let at = sim.place(window);
        let screen = at.screen;
        let paint = |sim: &Simulated, system| {
            sim.paint(&Scene::new(), at, 1.0, window, Brightness::Dark, system)
        };

        assert!(keyboard_panel(&paint(&sim, Brightness::Light), screen).is_none());

        sim.keyboard = true;
        let (panel, light) = keyboard_panel(&paint(&sim, Brightness::Light), screen).unwrap();
        assert_eq!(panel.height, 336.0);
        assert_eq!(panel.height, sim.insets().view_insets.bottom);
        let (_, dark) = keyboard_panel(&paint(&sim, Brightness::Dark), screen).unwrap();
        assert!(light.r > 0.5 && dark.r < 0.5, "{light:?} {dark:?}");

        sim.keyboard = false;
        sim.keyboard_held = true;
        assert!(sim.keyboard_up());
        assert_eq!(sim.insets().view_insets.bottom, 336.0);
        assert!(keyboard_panel(&paint(&sim, Brightness::Light), screen).is_some());
    }
}
