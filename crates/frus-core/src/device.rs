//! Device profiles — what a phone or a tablet looks like to an application, as data.
//!
//! A [`DeviceProfile`] describes a device closely enough for a desktop window to stand in
//! for it: the screen's logical size and density, the corners and the cutout that take
//! part of it, the areas the system keeps (status bar, home indicator) in each
//! orientation, how high the soft keyboard comes, the platform whose conventions it
//! follows, and the outline of the device drawn around the screen.
//!
//! It holds no behaviour: the shell reads a profile to size the window, inject the
//! insets and paint the outline. The built-in profiles are [`DeviceProfile::builtin`];
//! an application or a developer can describe any other device the same way.
//!
//! A simulated device is a **preview**: it gets the layout, the safe areas, the
//! keyboard and the platform's conventions right, not the device's own fonts, graphics
//! driver, native services or speed.

use crate::{Insets, Orientation, Rect, Size, TargetPlatform, WindowInsets};

/// A device a desktop window can stand in for: its screen, the areas its system keeps,
/// its keyboard, its platform and its outline.
///
/// Everything is in **logical** pixels and given for the device held **upright**
/// (portrait); [`Orientation::Landscape`] is the device turned a quarter turn
/// counter-clockwise, so that what was at the top of the screen — the cutout — is on the
/// left.
#[derive(Clone, Debug, PartialEq)]
pub struct DeviceProfile {
    /// What `frus run -d` and `frus devices` call it: lowercase, no spaces
    /// (`"iphone-15"`).
    pub id: String,
    /// The name shown to people (`"iPhone 15"`).
    pub name: String,
    /// The platform whose conventions the application follows on it.
    pub platform: TargetPlatform,
    /// The screen held upright, in logical pixels.
    pub screen: Size,
    /// Physical pixels per logical pixel.
    pub pixel_ratio: f32,
    /// The radius of the screen's own rounded corners; 0 for a square screen.
    pub screen_radius: f32,
    /// The part of the screen taken by the camera, held upright.
    pub cutout: Cutout,
    /// The system's areas and the keyboard, held upright.
    pub portrait: DeviceInsets,
    /// The system's areas and the keyboard, turned on its side.
    pub landscape: DeviceInsets,
    /// The outline drawn around the screen.
    pub frame: DeviceFrame,
}

/// The areas of the screen a device's system keeps, in one orientation.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DeviceInsets {
    /// What the status bar, the cutout and the home indicator take, keyboard down.
    pub safe: Insets,
    /// How high the soft keyboard comes, measured from the bottom edge of the screen
    /// (so including the home indicator it covers).
    pub keyboard: f32,
}

/// The part of a screen taken by the camera, described for the device held upright.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Cutout {
    /// The screen is whole.
    None,
    /// A notch cut into the top edge, centred: it touches the edge.
    Notch {
        /// Across the screen.
        width: f32,
        /// From the top edge down.
        height: f32,
        /// The radius of its lower corners.
        radius: f32,
    },
    /// A pill floating below the top edge, centred.
    Island {
        /// Across the screen.
        width: f32,
        /// Its height; the pill's radius is half of it.
        height: f32,
        /// From the top edge of the screen to the top of the pill.
        top: f32,
    },
    /// A round hole below the top edge, centred.
    Hole {
        /// Its diameter.
        diameter: f32,
        /// From the top edge of the screen to the top of the hole.
        top: f32,
    },
}

/// The outline of a device, drawn around its screen.
#[derive(Clone, Debug, PartialEq)]
pub struct DeviceFrame {
    /// How far the body reaches past the screen on each side, held upright.
    pub bezel: Insets,
    /// The radius of the body's corners.
    pub radius: f32,
    /// A round home button below the screen, centred in the bottom bezel.
    pub home_button: bool,
    /// The buttons on the body's edges, held upright.
    pub buttons: Vec<SideButton>,
}

/// A button on the edge of a device's body.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SideButton {
    /// Which edge it is on, held upright.
    pub edge: ButtonEdge,
    /// From the top of the body to the top of the button.
    pub top: f32,
    /// How long it is along the edge.
    pub length: f32,
}

/// The left or the right edge of a device held upright.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ButtonEdge {
    /// The left edge.
    Left,
    /// The right edge.
    Right,
}

impl DeviceProfile {
    /// The screen in `orientation`, in logical pixels.
    pub fn screen_size(&self, orientation: Orientation) -> Size {
        match orientation {
            Orientation::Portrait => self.screen,
            Orientation::Landscape => Size::new(self.screen.height, self.screen.width),
        }
    }

    /// The screen in `orientation`, in physical pixels.
    pub fn physical_size(&self, orientation: Orientation) -> Size {
        let size = self.screen_size(orientation);
        Size::new(
            (size.width * self.pixel_ratio).round(),
            (size.height * self.pixel_ratio).round(),
        )
    }

    /// The system's areas and the keyboard in `orientation`.
    pub fn insets_in(&self, orientation: Orientation) -> DeviceInsets {
        match orientation {
            Orientation::Portrait => self.portrait,
            Orientation::Landscape => self.landscape,
        }
    }

    /// The insets an application on this device is told about in `orientation`, with
    /// the soft keyboard up or down — split as a real device's shell splits them.
    pub fn window_insets(&self, orientation: Orientation, keyboard: bool) -> WindowInsets {
        let insets = self.insets_in(orientation);
        let mut current = insets.safe;
        if keyboard {
            current.bottom = current.bottom.max(insets.keyboard);
        }
        WindowInsets::from_baseline(insets.safe, current)
    }

    /// Where the cutout is on the screen in `orientation`, in logical pixels from the
    /// screen's top-left corner; `None` for a whole screen.
    ///
    /// Turned on its side, the cutout is on the left edge, centred top to bottom.
    pub fn cutout_rect(&self, orientation: Orientation) -> Option<Rect> {
        let (width, height, top) = match self.cutout {
            Cutout::None => return None,
            Cutout::Notch { width, height, .. } => (width, height, 0.0),
            Cutout::Island { width, height, top } => (width, height, top),
            Cutout::Hole { diameter, top } => (diameter, diameter, top),
        };
        let across = (self.screen.width - width) / 2.0;
        Some(match orientation {
            Orientation::Portrait => Rect::new(across, top, width, height),
            Orientation::Landscape => Rect::new(top, across, height, width),
        })
    }

    /// The whole device — screen and body — in `orientation`, in logical pixels.
    pub fn outer_size(&self, orientation: Orientation) -> Size {
        let b = self.frame.bezel;
        let upright = Size::new(
            self.screen.width + b.left + b.right,
            self.screen.height + b.top + b.bottom,
        );
        match orientation {
            Orientation::Portrait => upright,
            Orientation::Landscape => Size::new(upright.height, upright.width),
        }
    }

    /// Where the screen sits inside [`outer_size`](Self::outer_size) in `orientation`.
    ///
    /// Turned counter-clockwise, the top bezel is on the left and the left bezel at the
    /// bottom.
    pub fn screen_rect(&self, orientation: Orientation) -> Rect {
        let b = self.frame.bezel;
        let size = self.screen_size(orientation);
        match orientation {
            Orientation::Portrait => Rect::new(b.left, b.top, size.width, size.height),
            Orientation::Landscape => Rect::new(b.top, b.right, size.width, size.height),
        }
    }

    /// The profile among `profiles` whose id or name is `wanted`, ignoring case.
    pub fn find<'a>(profiles: &'a [DeviceProfile], wanted: &str) -> Option<&'a DeviceProfile> {
        profiles
            .iter()
            .find(|p| p.id.eq_ignore_ascii_case(wanted) || p.name.eq_ignore_ascii_case(wanted))
    }

    /// The devices frus knows without being told: phones and a tablet for iOS, and a
    /// phone for Android.
    ///
    /// The screens, densities, cutouts and safe areas are the devices' published
    /// figures. The keyboard heights are typical ones — they change with the keyboard,
    /// the language and the suggestions bar — and the outlines are frus's own drawing,
    /// not a picture of any device.
    pub fn builtin() -> Vec<DeviceProfile> {
        let island = Cutout::Island {
            width: 126.0,
            height: 37.0,
            top: 11.0,
        };
        let island_landscape = DeviceInsets {
            safe: Insets::new(0.0, 59.0, 21.0, 59.0),
            keyboard: 209.0,
        };
        let iphone_buttons = vec![
            SideButton {
                edge: ButtonEdge::Left,
                top: 120.0,
                length: 32.0,
            },
            SideButton {
                edge: ButtonEdge::Left,
                top: 180.0,
                length: 62.0,
            },
            SideButton {
                edge: ButtonEdge::Left,
                top: 256.0,
                length: 62.0,
            },
            SideButton {
                edge: ButtonEdge::Right,
                top: 210.0,
                length: 98.0,
            },
        ];
        vec![
            DeviceProfile {
                id: "iphone-se".into(),
                name: "iPhone SE".into(),
                platform: TargetPlatform::Ios,
                screen: Size::new(375.0, 667.0),
                pixel_ratio: 2.0,
                screen_radius: 0.0,
                cutout: Cutout::None,
                portrait: DeviceInsets {
                    safe: Insets::new(20.0, 0.0, 0.0, 0.0),
                    keyboard: 260.0,
                },
                landscape: DeviceInsets {
                    safe: Insets::ZERO,
                    keyboard: 200.0,
                },
                frame: DeviceFrame {
                    bezel: Insets::new(96.0, 18.0, 96.0, 18.0),
                    radius: 58.0,
                    home_button: true,
                    buttons: vec![
                        SideButton {
                            edge: ButtonEdge::Left,
                            top: 100.0,
                            length: 30.0,
                        },
                        SideButton {
                            edge: ButtonEdge::Left,
                            top: 160.0,
                            length: 56.0,
                        },
                        SideButton {
                            edge: ButtonEdge::Left,
                            top: 230.0,
                            length: 56.0,
                        },
                        SideButton {
                            edge: ButtonEdge::Right,
                            top: 160.0,
                            length: 56.0,
                        },
                    ],
                },
            },
            DeviceProfile {
                id: "iphone-15".into(),
                name: "iPhone 15".into(),
                platform: TargetPlatform::Ios,
                screen: Size::new(393.0, 852.0),
                pixel_ratio: 3.0,
                screen_radius: 55.0,
                cutout: island,
                portrait: DeviceInsets {
                    safe: Insets::new(59.0, 0.0, 34.0, 0.0),
                    keyboard: 336.0,
                },
                landscape: island_landscape,
                frame: DeviceFrame {
                    bezel: Insets::new(14.0, 14.0, 14.0, 14.0),
                    radius: 69.0,
                    home_button: false,
                    buttons: iphone_buttons.clone(),
                },
            },
            DeviceProfile {
                id: "iphone-15-pro-max".into(),
                name: "iPhone 15 Pro Max".into(),
                platform: TargetPlatform::Ios,
                screen: Size::new(430.0, 932.0),
                pixel_ratio: 3.0,
                screen_radius: 55.0,
                cutout: island,
                portrait: DeviceInsets {
                    safe: Insets::new(59.0, 0.0, 34.0, 0.0),
                    keyboard: 346.0,
                },
                landscape: island_landscape,
                frame: DeviceFrame {
                    bezel: Insets::new(14.0, 14.0, 14.0, 14.0),
                    radius: 69.0,
                    home_button: false,
                    buttons: iphone_buttons,
                },
            },
            DeviceProfile {
                id: "ipad".into(),
                name: "iPad".into(),
                platform: TargetPlatform::Ios,
                screen: Size::new(820.0, 1180.0),
                pixel_ratio: 2.0,
                screen_radius: 18.0,
                cutout: Cutout::None,
                portrait: DeviceInsets {
                    safe: Insets::new(24.0, 0.0, 20.0, 0.0),
                    keyboard: 314.0,
                },
                landscape: DeviceInsets {
                    safe: Insets::new(24.0, 0.0, 20.0, 0.0),
                    keyboard: 398.0,
                },
                frame: DeviceFrame {
                    bezel: Insets::new(26.0, 26.0, 26.0, 26.0),
                    radius: 44.0,
                    home_button: false,
                    buttons: vec![SideButton {
                        edge: ButtonEdge::Right,
                        top: 60.0,
                        length: 90.0,
                    }],
                },
            },
            DeviceProfile {
                id: "android-phone".into(),
                name: "Android phone".into(),
                platform: TargetPlatform::Android,
                // 1080 × 2400 physical pixels at 420 dpi: the common shape of a
                // mid-range phone.
                screen: Size::new(1080.0 / 2.625, 2400.0 / 2.625),
                pixel_ratio: 2.625,
                screen_radius: 32.0,
                cutout: Cutout::Hole {
                    diameter: 24.0,
                    top: 12.0,
                },
                portrait: DeviceInsets {
                    safe: Insets::new(48.0, 0.0, 24.0, 0.0),
                    keyboard: 300.0,
                },
                landscape: DeviceInsets {
                    safe: Insets::new(0.0, 0.0, 24.0, 48.0),
                    keyboard: 220.0,
                },
                frame: DeviceFrame {
                    bezel: Insets::new(12.0, 12.0, 12.0, 12.0),
                    radius: 44.0,
                    home_button: false,
                    buttons: vec![
                        SideButton {
                            edge: ButtonEdge::Right,
                            top: 170.0,
                            length: 56.0,
                        },
                        SideButton {
                            edge: ButtonEdge::Right,
                            top: 250.0,
                            length: 100.0,
                        },
                    ],
                },
            },
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn inside(inner: Rect, outer: Rect) -> bool {
        inner.x >= outer.x
            && inner.y >= outer.y
            && inner.x + inner.width <= outer.x + outer.width + 1e-3
            && inner.y + inner.height <= outer.y + outer.height + 1e-3
    }

    /// **The built-in list can be relied on**: one id each, written as a command line
    /// wants it, and found by either of its names.
    #[test]
    fn every_builtin_profile_has_its_own_id() {
        let profiles = DeviceProfile::builtin();
        for (i, p) in profiles.iter().enumerate() {
            assert!(
                p.id.chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-'),
                "{} is not a command-line id",
                p.id
            );
            assert!(profiles[..i].iter().all(|q| q.id != p.id), "{} twice", p.id);
            assert_eq!(DeviceProfile::find(&profiles, &p.id), Some(p));
            assert_eq!(
                DeviceProfile::find(&profiles, &p.name.to_uppercase()),
                Some(p)
            );
        }
        assert_eq!(DeviceProfile::find(&profiles, "nokia-3310"), None);
    }

    /// **The figures describe a device that can exist**: whole physical pixels, safe
    /// areas and a keyboard that fit on the screen, a cutout inside the area kept for it
    /// — upright and on its side.
    #[test]
    fn every_builtin_profile_is_consistent() {
        for p in DeviceProfile::builtin() {
            let upright = p.physical_size(Orientation::Portrait);
            let exact = Size::new(
                p.screen.width * p.pixel_ratio,
                p.screen.height * p.pixel_ratio,
            );
            assert!(
                (upright.width - exact.width).abs() < 0.01
                    && (upright.height - exact.height).abs() < 0.01,
                "{}: {exact:?} is not whole pixels",
                p.id
            );
            for o in [Orientation::Portrait, Orientation::Landscape] {
                let size = p.screen_size(o);
                let insets = p.insets_in(o);
                let s = insets.safe;
                assert!(s.left + s.right < size.width, "{} {o:?}", p.id);
                assert!(s.top + s.bottom < size.height, "{} {o:?}", p.id);
                assert!(
                    insets.keyboard >= s.bottom && insets.keyboard < size.height - s.top,
                    "{} {o:?}: keyboard {}",
                    p.id,
                    insets.keyboard
                );
                if let Some(cut) = p.cutout_rect(o) {
                    assert!(inside(cut, Rect::new(0.0, 0.0, size.width, size.height)));
                    match o {
                        Orientation::Portrait => assert!(cut.y + cut.height <= s.top, "{}", p.id),
                        Orientation::Landscape => {
                            assert!(cut.x + cut.width <= s.left, "{}", p.id)
                        }
                    }
                }
                let outer = p.outer_size(o);
                let screen = p.screen_rect(o);
                assert!(inside(
                    screen,
                    Rect::new(0.0, 0.0, outer.width, outer.height)
                ));
                assert_eq!((screen.width, screen.height), (size.width, size.height));
            }
            assert!(p.screen_radius <= p.frame.radius, "{}", p.id);
        }
    }

    /// **Turning the device swaps the screen and moves the cutout to the left.**
    #[test]
    fn landscape_is_a_quarter_turn_counter_clockwise() {
        let profiles = DeviceProfile::builtin();
        let p = DeviceProfile::find(&profiles, "iphone-15").unwrap();
        assert_eq!(
            p.screen_size(Orientation::Landscape),
            Size::new(852.0, 393.0)
        );
        assert_eq!(
            p.physical_size(Orientation::Landscape),
            Size::new(2556.0, 1179.0)
        );
        assert_eq!(
            p.cutout_rect(Orientation::Portrait),
            Some(Rect::new(133.5, 11.0, 126.0, 37.0))
        );
        assert_eq!(
            p.cutout_rect(Orientation::Landscape),
            Some(Rect::new(11.0, 133.5, 37.0, 126.0))
        );
        assert_eq!(
            p.outer_size(Orientation::Landscape),
            Size::new(880.0, 421.0)
        );
        assert_eq!(
            p.screen_rect(Orientation::Landscape),
            Rect::new(14.0, 14.0, 852.0, 393.0)
        );

        let se = DeviceProfile::find(&profiles, "iphone-se").unwrap();
        assert_eq!(se.cutout_rect(Orientation::Portrait), None);
        // The top bezel goes to the left, the left one to the bottom.
        assert_eq!(
            se.screen_rect(Orientation::Landscape),
            Rect::new(96.0, 18.0, 667.0, 375.0)
        );
    }

    /// **The keyboard is reported as a device reports it**: it covers the home
    /// indicator, so nothing pads the bottom twice, and the bars that do not move stay
    /// in `view_padding`.
    #[test]
    fn the_keyboard_moves_the_view_insets_only() {
        let profiles = DeviceProfile::builtin();
        let p = DeviceProfile::find(&profiles, "iphone-15").unwrap();

        let down = p.window_insets(Orientation::Portrait, false);
        assert_eq!(down, WindowInsets::bars(Insets::new(59.0, 0.0, 34.0, 0.0)));

        let up = p.window_insets(Orientation::Portrait, true);
        assert_eq!(up.view_insets, Insets::new(0.0, 0.0, 336.0, 0.0));
        assert_eq!(up.padding, Insets::new(59.0, 0.0, 0.0, 0.0));
        assert_eq!(up.view_padding, Insets::new(59.0, 0.0, 34.0, 0.0));
        assert_eq!(up.safe(), Insets::new(59.0, 0.0, 336.0, 0.0));

        let side = p.window_insets(Orientation::Landscape, true);
        assert_eq!(side.view_insets.bottom, 209.0);
        assert_eq!(side.padding, Insets::new(0.0, 59.0, 0.0, 59.0));
    }
}
