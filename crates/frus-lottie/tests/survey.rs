//! **A survey over real animations**, run by hand:
//!
//! ```sh
//! LOTTIE_SAMPLES=/path/to/a/folder/of/json cargo test -p frus-lottie --test survey -- --ignored --nocapture
//! ```
//!
//! Each file is parsed and painted at five moments. Nothing may panic; what does not parse,
//! and what is not drawn, is counted and named. lottie-android's snapshot tests
//! (`snapshot-tests/src/main/assets/Tests`, Apache-2.0) are a good folder to point it at: one
//! small file per feature.

use frus_core::{Alignment, AlignmentGeometry, BoxFit, Primitive, Rect, Scene};
use frus_lottie::Animation;

fn count(primitives: &[Primitive]) -> usize {
    primitives
        .iter()
        .map(|p| match p {
            Primitive::Layer { primitives, .. } => count(primitives),
            _ => 1,
        })
        .sum()
}

#[test]
#[ignore = "needs LOTTIE_SAMPLES, a folder of Lottie files"]
fn every_animation_in_the_folder_paints() {
    let Ok(folder) = std::env::var("LOTTIE_SAMPLES") else {
        eprintln!("LOTTIE_SAMPLES is not set: nothing to survey");
        return;
    };
    let mut files: Vec<_> = std::fs::read_dir(&folder)
        .expect("the folder")
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e == "json"))
        .collect();
    files.sort();
    let (mut painted, mut unparsed, mut empty) = (0, Vec::new(), Vec::new());
    let mut panicked: Vec<String> = Vec::new();
    let mut unsupported: std::collections::BTreeMap<&str, Vec<String>> = Default::default();
    for file in &files {
        let name = file.file_name().unwrap().to_string_lossy().into_owned();
        let bytes = std::fs::read(file).unwrap();
        // Tests unwind (release builds of applications abort): a panic in the parser or the
        // renderer is counted and named here rather than stopping the survey.
        let animation = match std::panic::catch_unwind(|| Animation::parse(&bytes)) {
            Ok(Ok(a)) => a,
            Ok(Err(why)) => {
                unparsed.push(format!("{name}: {}", why.message()));
                continue;
            }
            Err(_) => {
                panicked.push(format!("{name} (parsing)"));
                continue;
            }
        };
        let mut drawn = 0;
        let mut frames_panicked = false;
        for progress in [0.0, 0.25, 0.5, 0.75, 1.0] {
            let mut scene = Scene::new();
            scene.set_clip(Rect::new(0.0, 0.0, 400.0, 400.0));
            let painted = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                animation.paint(
                    &mut scene,
                    Rect::new(0.0, 0.0, 400.0, 400.0),
                    BoxFit::Contain,
                    AlignmentGeometry::Physical(Alignment::CENTER),
                    progress,
                    1.0,
                )
            }));
            if painted.is_err() {
                frames_panicked = true;
                break;
            }
            drawn += count(scene.primitives());
        }
        if frames_panicked {
            panicked.push(format!("{name} (painting)"));
            continue;
        }
        if drawn == 0 {
            empty.push(name.clone());
        }
        for what in animation.unsupported() {
            unsupported.entry(what).or_default().push(name.clone());
        }
        painted += 1;
    }
    println!(
        "{} files: {painted} painted, {} not parsed",
        files.len(),
        unparsed.len()
    );
    for line in &unparsed {
        println!("  not parsed: {line}");
    }
    println!("panicked ({}): {panicked:?}", panicked.len());
    println!("painted nothing at any of the five moments: {empty:?}");
    for (what, names) in &unsupported {
        println!("not drawn, {what}: {} ({names:?})", names.len());
    }
}
