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

    /// A device a developer describes, as `key=value` pairs separated by `;`
    /// (milestone 727). The keys are those of a project's `[[package.metadata.frus.devices]]`,
    /// which `frus run` hands over this way; a list is written `a,b,c`.
    ///
    /// Required: `id`, `platform` (`ios` or `android`), `width`, `height` (logical, upright)
    /// and `ratio`. The rest has a default: `name` (the id), `corner-radius` (0), `safe-area`
    /// (`top,right,bottom,left`, none), `landscape-safe-area` (the upright one turned with the
    /// device), `keyboard` (two fifths of the height), `landscape-keyboard` (three fifths of
    /// the width), `cutout` (`none`, `notch`, `island` or `hole`) with `cutout-size`
    /// (`width,height`) and `cutout-top`. The outline is a plain body around the screen.
    ///
    /// An unknown key is an error rather than ignored: a misspelt one would otherwise leave
    /// its default in place without a word.
    pub fn parse(spec: &str) -> Result<DeviceProfile, String> {
        const KEYS: [&str; 14] = [
            "id",
            "name",
            "platform",
            "width",
            "height",
            "ratio",
            "corner-radius",
            "safe-area",
            "landscape-safe-area",
            "keyboard",
            "landscape-keyboard",
            "cutout",
            "cutout-size",
            "cutout-top",
        ];
        let mut pairs: Vec<(&str, &str)> = Vec::new();
        for part in spec.split(';').map(str::trim).filter(|p| !p.is_empty()) {
            let (key, value) = part
                .split_once('=')
                .ok_or_else(|| format!("`{part}` is not `key = value`"))?;
            let key = key.trim();
            if !KEYS.contains(&key) {
                return Err(format!(
                    "`{key}` is not a device key; the keys: {}",
                    KEYS.join(", ")
                ));
            }
            pairs.push((key, value.trim()));
        }
        let get = |key: &str| pairs.iter().rev().find(|(k, _)| *k == key).map(|(_, v)| *v);
        let id = get("id")
            .filter(|v| !v.is_empty())
            .ok_or("a device needs an `id`")?;
        let numbers = |key: &str, count: usize| -> Result<Option<Vec<f32>>, String> {
            let Some(value) = get(key) else {
                return Ok(None);
            };
            let parsed: Result<Vec<f32>, _> =
                value.split(',').map(|n| n.trim().parse::<f32>()).collect();
            match parsed {
                Ok(numbers) if numbers.len() == count && numbers.iter().all(|n| n.is_finite()) => {
                    Ok(Some(numbers))
                }
                _ if count == 1 => Err(format!("{id}: `{key}` is {value}, not a number")),
                _ => Err(format!("{id}: `{key}` is {value}, not {count} numbers")),
            }
        };
        let number = |key: &str| numbers(key, 1).map(|n| n.map(|n| n[0]));
        let positive = |key: &str| -> Result<f32, String> {
            match number(key)? {
                Some(n) if n > 0.0 => Ok(n),
                Some(n) => Err(format!("{id}: `{key}` is {n}; it must be more than 0")),
                None => Err(format!("{id}: a device needs a `{key}`")),
            }
        };
        let insets =
            |key: &str| numbers(key, 4).map(|n| n.map(|n| Insets::new(n[0], n[1], n[2], n[3])));

        if !id
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        {
            return Err(format!(
                "`{id}` is not a device id: lowercase letters, digits and `-` only"
            ));
        }
        let platform = match get("platform").map(str::to_ascii_lowercase).as_deref() {
            Some("ios") => TargetPlatform::Ios,
            Some("android") => TargetPlatform::Android,
            Some(other) => return Err(format!("{id}: `platform` is {other}; ios or android")),
            None => return Err(format!("{id}: a device needs a `platform`, ios or android")),
        };
        let (width, height, pixel_ratio) =
            (positive("width")?, positive("height")?, positive("ratio")?);
        let screen_radius = number("corner-radius")?.unwrap_or(0.0).max(0.0);
        let safe = insets("safe-area")?.unwrap_or(Insets::ZERO);
        // Turned counter-clockwise, what was at the top is on the left, and so on round.
        let landscape_safe = insets("landscape-safe-area")?.unwrap_or(Insets::new(
            safe.right,
            safe.bottom,
            safe.left,
            safe.top,
        ));
        let cutout = match get("cutout").unwrap_or("none") {
            "none" => Cutout::None,
            kind => {
                let size = numbers("cutout-size", 2)?
                    .ok_or_else(|| format!("{id}: a `{kind}` cutout needs a `cutout-size`"))?;
                let top = number("cutout-top")?.unwrap_or(0.0);
                match kind {
                    "notch" => Cutout::Notch {
                        width: size[0],
                        height: size[1],
                        radius: size[1] / 3.0,
                    },
                    "island" => Cutout::Island {
                        width: size[0],
                        height: size[1],
                        top,
                    },
                    "hole" => Cutout::Hole {
                        diameter: size[0],
                        top,
                    },
                    other => {
                        return Err(format!(
                            "{id}: `cutout` is {other}; none, notch, island or hole"
                        ))
                    }
                }
            }
        };
        Ok(DeviceProfile {
            id: id.to_string(),
            name: get("name")
                .filter(|n| !n.is_empty())
                .unwrap_or(id)
                .to_string(),
            platform,
            screen: Size::new(width, height),
            pixel_ratio,
            screen_radius,
            cutout,
            portrait: DeviceInsets {
                safe,
                keyboard: number("keyboard")?.unwrap_or(height * 2.0 / 5.0),
            },
            landscape: DeviceInsets {
                safe: landscape_safe,
                keyboard: number("landscape-keyboard")?.unwrap_or(width * 3.0 / 5.0),
            },
            frame: DeviceFrame {
                bezel: Insets::new(12.0, 12.0, 12.0, 12.0),
                radius: screen_radius + 12.0,
                home_button: false,
                buttons: Vec::new(),
            },
        })
    }

    /// The devices frus knows without being told: phones and a tablet for iOS, and a
    /// phone for Android.
    ///
    /// The iOS screens, densities, cutouts and safe areas are the devices' published
    /// figures; the Android phone's are what Android reports for its shape, measured. The
    /// keyboard heights are typical ones — they change with the keyboard, the language and
    /// the suggestions bar — and the outlines are frus's own drawing, not a picture of any
    /// device.
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
                // mid-range phone. Its bars, hole and keyboard are what Android 16 reports
                // for that shape with a punch hole, gesture navigation and Gboard, in
                // physical pixels (milestone 723); the hole is put in the middle, where most
                // phones have it.
                screen: Size::new(1080.0 / 2.625, 2400.0 / 2.625),
                pixel_ratio: 2.625,
                screen_radius: 32.0,
                cutout: Cutout::Hole {
                    diameter: 88.0 / 2.625,
                    top: 39.0 / 2.625,
                },
                portrait: DeviceInsets {
                    // The status bar is as tall as the band the hole takes.
                    safe: Insets::new(136.0 / 2.625, 0.0, 63.0 / 2.625, 0.0),
                    keyboard: 883.0 / 2.625,
                },
                landscape: DeviceInsets {
                    // On its side the status bar stays, the hole's band goes to the left.
                    safe: Insets::new(63.0 / 2.625, 0.0, 63.0 / 2.625, 136.0 / 2.625),
                    keyboard: 686.0 / 2.625,
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

    /// **A developer's own device reads as it is written** (milestone 727): every key, then
    /// only the required ones with the rest defaulted.
    #[test]
    fn a_described_device_is_read() {
        let full = DeviceProfile::parse(
            "id=pixel-9; name=Pixel 9; platform=android; width=412; height=924; ratio=2.625; \
             corner-radius=40; safe-area=52,0,24,0; landscape-safe-area=24,0,24,52; \
             keyboard=336; landscape-keyboard=260; cutout=hole; cutout-size=34,34; cutout-top=15",
        )
        .unwrap();
        assert_eq!(
            (full.id.as_str(), full.name.as_str()),
            ("pixel-9", "Pixel 9")
        );
        assert_eq!(full.platform, TargetPlatform::Android);
        assert_eq!(
            (full.screen, full.pixel_ratio),
            (Size::new(412.0, 924.0), 2.625)
        );
        assert_eq!(full.portrait.safe, Insets::new(52.0, 0.0, 24.0, 0.0));
        assert_eq!(full.landscape.safe, Insets::new(24.0, 0.0, 24.0, 52.0));
        assert_eq!(
            (full.portrait.keyboard, full.landscape.keyboard),
            (336.0, 260.0)
        );
        assert_eq!(
            full.cutout,
            Cutout::Hole {
                diameter: 34.0,
                top: 15.0
            }
        );
        assert_eq!(full.frame.radius, 52.0);

        let bare =
            DeviceProfile::parse("id=tab;platform=IOS;width=800;height=1200;ratio=2").unwrap();
        assert_eq!(bare.name, "tab");
        assert_eq!(bare.platform, TargetPlatform::Ios);
        assert_eq!(bare.cutout, Cutout::None);
        assert_eq!(bare.portrait.keyboard, 480.0);
        assert_eq!(bare.landscape.keyboard, 480.0);

        // On its side, the upright safe area turns with the device: its top goes to the left.
        let turned = DeviceProfile::parse(
            "id=t;platform=ios;width=390;height=844;ratio=3;safe-area=47,0,34,0",
        )
        .unwrap();
        assert_eq!(turned.landscape.safe, Insets::new(0.0, 34.0, 0.0, 47.0));
    }

    /// **A mistake is named**, rather than a default quietly taking its place.
    #[test]
    fn a_wrong_description_says_what_is_wrong() {
        let base = "id=p;platform=android;width=400;height=800;ratio=2";
        let error = |extra: &str| DeviceProfile::parse(&format!("{base};{extra}")).unwrap_err();
        assert!(error("heigth=900").contains("`heigth` is not a device key"));
        assert!(error("keyboard=tall").contains("`keyboard` is tall, not a number"));
        assert!(error("safe-area=1,2,3").contains("not 4 numbers"));
        assert!(error("cutout=island").contains("needs a `cutout-size`"));
        assert!(error("cutout=slot;cutout-size=1,1").contains("none, notch, island or hole"));
        assert!(error("platform=windows").contains("ios or android"));
        assert!(error("width=0").contains("more than 0"));
        assert!(
            DeviceProfile::parse("platform=ios;width=1;height=1;ratio=1")
                .unwrap_err()
                .contains("needs an `id`")
        );
        assert!(
            DeviceProfile::parse("id=Big Phone;platform=ios;width=1;height=1;ratio=1")
                .unwrap_err()
                .contains("not a device id")
        );
        assert!(DeviceProfile::parse("id=p;width=1;height=1;ratio=1")
            .unwrap_err()
            .contains("needs a `platform`"));
        assert!(DeviceProfile::parse("id=p;platform=ios;height=1;ratio=1")
            .unwrap_err()
            .contains("needs a `width`"));
        assert!(DeviceProfile::parse("id=p;nonsense")
            .unwrap_err()
            .contains("key = value"));
    }
}
