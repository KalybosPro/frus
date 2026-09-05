//! The tool that renders the README's pictures, behind the `shots` feature.
//!
//! ```sh
//! cargo run -p frus-demo --features shots --bin shots -- docs/media
//! ```
//!
//! The pictures are **rendered**, not photographed: every frame here goes through
//! the same pipeline a window does, offscreen, at a size chosen for the page rather
//! than for whatever monitor was to hand. So they can be regenerated after a change
//! instead of slowly going out of date, and there is no screenshot in the repository
//! that nobody knows how to reproduce.
//!
//! The one exception is the Android picture, which is a real screenshot of a real
//! phone — a rendering could not honestly claim what it is there to claim.

use std::path::Path;

use frus_test::Stage;

use crate::prelude::*;
use crate::TodoApp;

/// One frame of the animation, in seconds. The GIF is written at the same rate.
const DT: f32 = 1.0 / 30.0;

/// A screen worth a picture: where to go, how big, and in which theme.
struct Shot {
    name: &'static str,
    route: Option<Route>,
    width: u32,
    height: u32,
    light: bool,
}

const SHOTS: &[Shot] = &[
    // The home screen, where most of the widget library is on show at once. Kept
    // close to the content's own width: the column is centred and a wider frame just
    // buys empty margins.
    Shot {
        name: "tasks",
        route: None,
        width: 900,
        height: 640,
        light: false,
    },
    Shot {
        name: "charts",
        route: Some(Route::Charts),
        width: 900,
        height: 640,
        light: false,
    },
    Shot {
        name: "board",
        route: Some(Route::Board),
        width: 900,
        height: 640,
        light: false,
    },
    Shot {
        name: "data",
        route: Some(Route::Data),
        width: 900,
        height: 640,
        light: false,
    },
    // The same application in the light theme, to make the point that the theme is
    // not a coat of paint over a dark design.
    Shot {
        name: "light",
        route: Some(Route::Settings),
        width: 900,
        height: 640,
        light: true,
    },
];

/// Renders every picture into `dir`.
pub fn write_previews(dir: &Path) -> anyhow::Result<()> {
    std::fs::create_dir_all(dir)?;
    for shot in SHOTS {
        write_shot(dir, shot)?;
    }
    write_transition_gif(dir)?;
    Ok(())
}

/// Builds the application, walks it to `shot.route`, lets every animation settle,
/// and writes one frame.
fn write_shot(dir: &Path, shot: &Shot) -> anyhow::Result<()> {
    let mut app = seeded_app(shot.light);
    if let Some(route) = shot.route {
        let _ = app.update(Msg::Push(route));
    }
    // A route change is a spring: settle it, rather than photographing the application
    // mid-thought. The theme's own crossing is the framework's since milestone 452, and
    // is not running here — `resolved_theme` below asks for the destination directly.
    for _ in 0..120 {
        if !app.tick(DT) {
            break;
        }
    }

    let (width, height) = (shot.width, shot.height);
    let theme = shot_theme(&app);
    let mut stage = Stage::new(width, height).theme(theme.clone());
    let root = MediaQuery::new(Size::new(width as f32, height as f32)).scope(|| app.view(&theme));
    stage.settle(root.as_ref());
    // Two settled frames: the first adopts every implicit target, the second draws
    // the tree that adoption produced.
    let Some(frame) = stage.render(root.as_ref()) else {
        anyhow::bail!("no GPU adapter: nothing can be rendered");
    };
    let path = dir.join(format!("{}.png", shot.name));
    frame.write_png(&path);
    println!("{} — {width}×{height}", path.display());
    Ok(())
}

/// The moving picture: a walk through four screens, each entered and left through
/// the framework's spring route transition.
///
/// A single transition is prettier in isolation, but this is the README's opening
/// image and it has one job — show that there is a widget library here and that it
/// moves. Breadth first, then the motion carries it.
fn write_transition_gif(dir: &Path) -> anyhow::Result<()> {
    const WIDTH: u32 = 600;
    const HEIGHT: u32 = 412;
    /// The simulation runs at `DT`; one frame in three reaches the GIF. This is the
    /// only real lever on the file's size — a GIF writes every frame whole — and at
    /// the top of a README the weight matters more than the last few frames per
    /// second.
    const KEEP_EVERY: usize = 3;
    /// Frames held on a screen once it has arrived, at the simulation's rate. These
    /// cost almost nothing: a held frame is identical to the one before it and is
    /// folded into its delay below.
    const HOLD: usize = 18;
    const MAX_FRAMES: usize = 90;

    // Three stops, not every screen. Each one costs two transitions, and the stills
    // below the GIF can show the rest for the price of a PNG.
    let stops = [Route::Charts, Route::Board, Route::Data];

    let mut app = seeded_app(false);
    let theme = app.theme();
    let mut stage = Stage::new(WIDTH, HEIGHT).theme(theme.clone());
    {
        let root =
            MediaQuery::new(Size::new(WIDTH as f32, HEIGHT as f32)).scope(|| app.view(&theme));
        stage.settle(root.as_ref());
    }

    let mut frames: Vec<Vec<u8>> = Vec::new();
    let capture = |app: &TodoApp, stage: &mut Stage, frames: &mut Vec<Vec<u8>>| {
        let theme = app.theme();
        stage.theme = theme.clone();
        let root =
            MediaQuery::new(Size::new(WIDTH as f32, HEIGHT as f32)).scope(|| app.view(&theme));
        stage.advance(root.as_ref(), DT);
        if let Some(frame) = stage.render(root.as_ref()) {
            frames.push(frame.rgba);
        }
    };
    // Runs the application's own clock until it says it has stopped moving,
    // capturing as it goes — so the picture is the transition, not a guess at it.
    let play = |app: &mut TodoApp, stage: &mut Stage, frames: &mut Vec<Vec<u8>>| {
        for _ in 0..MAX_FRAMES {
            capture(app, stage, frames);
            if !app.tick(DT) {
                break;
            }
        }
    };
    let hold = |app: &mut TodoApp, stage: &mut Stage, frames: &mut Vec<Vec<u8>>| {
        for _ in 0..HOLD {
            app.tick(DT);
            capture(app, stage, frames);
        }
    };

    hold(&mut app, &mut stage, &mut frames);
    for route in stops {
        let _ = app.update(Msg::Push(route));
        play(&mut app, &mut stage, &mut frames);
        hold(&mut app, &mut stage, &mut frames);
        let _ = app.update(Msg::Pop);
        play(&mut app, &mut stage, &mut frames);
    }
    // Back where it started, so the loop closes instead of cutting.
    hold(&mut app, &mut stage, &mut frames);

    let path = dir.join("tour.gif");
    let mut file = std::io::BufWriter::new(std::fs::File::create(&path)?);
    let mut encoder = gif::Encoder::new(&mut file, WIDTH as u16, HEIGHT as u16, &[])?;
    encoder.set_repeat(gif::Repeat::Infinite)?;
    // A frame identical to the one before it is not written again: its time is added
    // to that one's delay instead. The tour spends half its length holding still on a
    // screen, and holding still is exactly what a GIF can express for free.
    let delay = (100.0 * DT * KEEP_EVERY as f32).round().max(1.0) as u16;
    let mut kept: Vec<(Vec<u8>, u16)> = Vec::new();
    for (i, rgba) in frames.into_iter().enumerate() {
        if i % KEEP_EVERY != 0 {
            continue;
        }
        match kept.last_mut() {
            Some((previous, held)) if *previous == rgba => *held += delay,
            _ => kept.push((rgba, delay)),
        }
    }

    let written = kept.len();
    for (mut rgba, held) in kept {
        // Speed 10 of 30: the palette is rebuilt per frame, and a flat interface
        // costs nothing visible to a faster quantiser.
        let mut frame = gif::Frame::from_rgba_speed(WIDTH as u16, HEIGHT as u16, &mut rgba, 10);
        // Centiseconds, which is all a GIF can express.
        frame.delay = held;
        encoder.write_frame(&frame)?;
    }
    drop(encoder);
    let bytes = std::fs::metadata(&path)?.len();
    println!(
        "{} — {WIDTH}×{HEIGHT}, {written} frames, {} KiB",
        path.display(),
        bytes / 1024
    );
    Ok(())
}

/// **The theme the application would be showing**, asked of the application itself.
///
/// The shell resolves this every frame from the platform's brightness and the reader's
/// contrast setting; nothing here has a platform, so it asks for a light one and lets the
/// application's own `theme_mode` — which this demonstration pins — decide.
fn shot_theme(app: &TodoApp) -> Theme {
    Application::resolved_theme(app, Brightness::Light, false)
}

/// The application as it starts, with its demonstration data, in the asked-for theme.
fn seeded_app(light: bool) -> TodoApp {
    let mut app = TodoApp::default();
    let _ = app.init();
    // `init` loads the persisted tasks asynchronously, which a rendering will not
    // wait for; seed the list directly so the picture never comes out empty.
    if app.todos.is_empty() {
        for (text, done) in [
            ("Read the design notes", true),
            ("Ship the Android build", false),
            ("Answer the issue about theming", false),
        ] {
            let id = app.next_id;
            app.next_id += 1;
            app.todos.push(Todo {
                id,
                text: text.to_string(),
                done,
            });
        }
    }
    if light != app.light {
        let _ = app.update(Msg::ToggleTheme);
    }
    app
}
