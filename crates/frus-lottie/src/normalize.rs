//! **A Lottie file brought to the form velato reads**, before velato sees it.
//!
//! velato reads the format as Bodymovin 5.5 and later write it. Files written before — and a
//! good share of the animations in circulation were — differ in ways that are mechanical to
//! undo: properties without their `"a"` flag, keyframes that carry their end value (`"e"`)
//! and leave the last keyframe a bare time. Some features velato does not handle at all, and a
//! few of those it does not refuse either: it panics on them, which in an application built
//! to abort on a panic is the application gone. Those are repaired where a meaning survives —
//! a missing rotation is no rotation, an unanimated split position is a position, an
//! unsupported blend is the normal one — and refused with an error where it does not.
//!
//! What is dropped or changed is named, for [`Animation::unsupported`](crate::Animation::unsupported).

use serde_json::{json, Map, Value};

/// Rewrites `root`, a parsed Lottie file, into the form velato reads. `notes` collects what
/// was dropped or changed; an error is what cannot be read at all.
pub(crate) fn normalize(root: &mut Value, notes: &mut Vec<&'static str>) -> Result<(), String> {
    properties(root);
    let Some(root) = root.as_object_mut() else {
        return Err("not a Lottie animation: the file is not a JSON object".to_string());
    };
    if let Some(Value::Array(layers)) = root.get_mut("layers") {
        normalize_layers(layers, notes)?;
    }
    if let Some(Value::Array(assets)) = root.get_mut("assets") {
        for asset in assets {
            if let Some(Value::Array(layers)) = asset.get_mut("layers") {
                normalize_layers(layers, notes)?;
            }
        }
    }
    Ok(())
}

fn note(notes: &mut Vec<&'static str>, what: &'static str) {
    if !notes.contains(&what) {
        notes.push(what);
    }
}

/// Every animatable property in the tree, given its `"a"` flag and modern keyframes.
///
/// A property is an object with a `"k"` and no `"ty"` (shape items have one): its `"k"` is a
/// number, an array of numbers, a Bézier (an object with `"v"`), or an array of keyframes
/// (objects with `"t"`). A gradient's `"g"` holds its property under `"k"` as an object, and is
/// left alone: its inner property is reached by the walk.
fn properties(value: &mut Value) {
    match value {
        Value::Object(map) => {
            if !map.contains_key("ty") {
                if let Some(k) = map.get("k") {
                    let keyframes = is_keyframes(k);
                    let static_value = !keyframes
                        && (k.is_number()
                            || k.as_array().is_some_and(|a| a.iter().all(Value::is_number))
                            || k.as_object().is_some_and(|o| o.contains_key("v")));
                    if keyframes || static_value {
                        map.entry("a")
                            .or_insert(json!(if keyframes { 1 } else { 0 }));
                    }
                    if keyframes {
                        if let Some(Value::Array(frames)) = map.get_mut("k") {
                            modern_keyframes(frames);
                        }
                    }
                }
            }
            for child in map.values_mut() {
                properties(child);
            }
        }
        Value::Array(items) => items.iter_mut().for_each(properties),
        _ => {}
    }
}

fn is_keyframes(k: &Value) -> bool {
    k.as_array()
        .and_then(|a| a.first())
        .is_some_and(|first| first.as_object().is_some_and(|o| o.contains_key("t")))
}

/// Old keyframes as modern ones: each keyframe's start value (`"s"`) is the previous one's end
/// (`"e"`) where it has none — the last keyframe of an old file is a bare time.
fn modern_keyframes(frames: &mut [Value]) {
    for i in 1..frames.len() {
        let missing = frames[i].get("s").is_none();
        if missing {
            if let Some(end) = frames[i - 1].get("e").cloned() {
                if let Some(frame) = frames[i].as_object_mut() {
                    frame.insert("s".to_string(), end);
                }
            }
        }
    }
}

fn normalize_layers(layers: &mut Vec<Value>, notes: &mut Vec<&'static str>) -> Result<(), String> {
    // Text layers (type 5): velato has none, and refuses the whole file for one.
    let before = layers.len();
    layers.retain(|layer| layer.get("ty").and_then(Value::as_u64) != Some(5));
    if layers.len() != before {
        note(notes, "text");
    }
    for layer in layers.iter_mut() {
        let Some(layer) = layer.as_object_mut() else {
            continue;
        };
        // A solid layer (type 1) is a rectangle of a colour: velato reads it and draws
        // nothing, so it becomes the shape layer that draws it.
        if layer.get("ty").and_then(Value::as_u64) == Some(1) {
            solid_as_shape(layer);
        }
        // The blends velato panics on: Add (16) and Hard Mix (17).
        if matches!(layer.get("bm").and_then(Value::as_u64), Some(16 | 17)) {
            layer.insert("bm".to_string(), json!(0));
            note(notes, "blend mode add or hard mix");
        }
        if let Some(Value::Object(ks)) = layer.get_mut("ks") {
            transform(ks, notes, true)?;
        }
        if let Some(Value::Array(shapes)) = layer.get_mut("shapes") {
            normalize_shapes(shapes, notes)?;
        }
    }
    Ok(())
}

/// A solid layer as a shape layer: a `sw` × `sh` rectangle from the layer's origin, filled
/// with `sc` (`#rrggbb`).
fn solid_as_shape(layer: &mut Map<String, Value>) {
    let size = |key: &str| layer.get(key).and_then(Value::as_f64).unwrap_or(0.0);
    let (w, h) = (size("sw"), size("sh"));
    let hex = layer
        .get("sc")
        .and_then(Value::as_str)
        .unwrap_or("#000000")
        .to_string();
    let channel = |at: usize| {
        hex.trim_start_matches('#')
            .get(at..at + 2)
            .and_then(|c| u8::from_str_radix(c, 16).ok())
            .map_or(0.0, |c| f64::from(c) / 255.0)
    };
    let fixed = |k: Value| json!({"a": 0, "k": k});
    layer.insert("ty".to_string(), json!(4));
    layer.insert(
        "shapes".to_string(),
        json!([{"ty": "gr", "nm": "solid", "it": [
            {"ty": "rc", "d": 1, "p": fixed(json!([w / 2.0, h / 2.0])),
             "s": fixed(json!([w, h])), "r": fixed(json!(0))},
            {"ty": "fl", "c": fixed(json!([channel(0), channel(2), channel(4), 1])),
             "o": fixed(json!(100)), "r": 1},
            {"ty": "tr", "p": fixed(json!([0, 0])), "a": fixed(json!([0, 0])),
             "s": fixed(json!([100, 100])), "r": fixed(json!(0)), "o": fixed(json!(100))}
        ]}]),
    );
}

fn normalize_shapes(shapes: &mut Vec<Value>, notes: &mut Vec<&'static str>) -> Result<(), String> {
    // A path with no vertices panics when it is drawn: it draws nothing anyway.
    shapes.retain(|shape| !empty_path(shape));
    for shape in shapes.iter_mut() {
        let Some(shape) = shape.as_object_mut() else {
            continue;
        };
        match shape.get("ty").and_then(Value::as_str) {
            Some("gr") => {
                if let Some(Value::Array(items)) = shape.get_mut("it") {
                    normalize_shapes(items, notes)?;
                }
            }
            Some("tr") => transform(shape, notes, false)?,
            _ => {}
        }
    }
    Ok(())
}

/// Whether `shape` is a path (`"sh"`) whose Bézier — or any keyframe's — has no vertex.
fn empty_path(shape: &Value) -> bool {
    if shape.get("ty").and_then(Value::as_str) != Some("sh") {
        return false;
    }
    let Some(k) = shape.get("ks").and_then(|ks| ks.get("k")) else {
        return false;
    };
    let empty = |bezier: &Value| {
        bezier
            .get("v")
            .and_then(Value::as_array)
            .is_some_and(|v| v.is_empty())
    };
    if is_keyframes(k) {
        k.as_array().unwrap().iter().any(|frame| {
            frame
                .get("s")
                .and_then(Value::as_array)
                .is_some_and(|s| s.iter().any(empty))
        })
    } else {
        empty(k)
    }
}

/// A layer's (`"ks"`) or a group's (`"tr"`) transform, in the form velato does not panic on.
fn transform(
    t: &mut Map<String, Value>,
    notes: &mut Vec<&'static str>,
    layer: bool,
) -> Result<(), String> {
    // Rotation: a 3D layer has `rx`, `ry` and `rz` and no `r`; the turn in the plane is `rz`.
    // velato panics on a rotation that is missing or split.
    let split_rotation = t.get("r").is_some_and(|r| r.get("x").is_some());
    if !t.contains_key("r") || split_rotation {
        let r = t
            .get("rz")
            .cloned()
            .unwrap_or_else(|| json!({"a": 0, "k": 0}));
        if t.contains_key("rx") || t.contains_key("ry") || split_rotation {
            note(notes, "3D rotation");
        }
        t.insert("r".to_string(), r);
    }
    // No position at all (`"ks": {}`): the origin, where velato refuses the file.
    t.entry("p")
        .or_insert_with(|| json!({"a": 0, "k": [0, 0, 0]}));
    // Split position: a group's (velato panics on it), and a layer's whose parts velato cannot
    // read. Unanimated, it is one position; animated, it is refused.
    if let Some(p) = t.get("p") {
        if p.get("s").and_then(Value::as_bool) == Some(true) || p.get("x").is_some() {
            let part = |axis: &str| -> Option<f64> {
                let k = p.get(axis)?.get("k")?;
                k.as_f64()
                    .or_else(|| k.as_array().and_then(|a| a.first()).and_then(Value::as_f64))
            };
            let animated = ["x", "y"].iter().any(|axis| {
                p.get(*axis)
                    .and_then(|v| v.get("k"))
                    .is_some_and(is_keyframes)
            });
            match (part("x"), part("y"), animated) {
                (Some(x), Some(y), false) => {
                    t.insert("p".to_string(), json!({"a": 0, "k": [x, y, 0]}));
                }
                _ if layer => {
                    // velato reads a layer's split position itself when its parts are well
                    // formed; leave it to it.
                }
                _ => {
                    return Err(
                        "a shape group's position is animated per axis, which is not supported"
                            .to_string(),
                    )
                }
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn normalized(source: Value) -> (Value, Vec<&'static str>) {
        let mut value = source;
        let mut notes = Vec::new();
        normalize(&mut value, &mut notes).expect("normalizes");
        (value, notes)
    }

    #[test]
    fn an_old_property_gets_its_animated_flag() {
        let (v, _) =
            normalized(json!({"layers":[{"ty":4,"ks":{"o":{"k":100},"p":{"k":[1,2,0]}}}]}));
        assert_eq!(v["layers"][0]["ks"]["o"]["a"], json!(0));
        assert_eq!(v["layers"][0]["ks"]["p"]["a"], json!(0));
    }

    #[test]
    fn old_keyframes_take_their_start_from_the_previous_end() {
        let (v, _) = normalized(json!({"layers":[{"ty":4,"ks":{"o":{"k":[
            {"t":0,"s":[0],"e":[100]},{"t":30}
        ]}}}]}));
        let o = &v["layers"][0]["ks"]["o"];
        assert_eq!(o["a"], json!(1));
        assert_eq!(o["k"][1]["s"], json!([100]));
    }

    #[test]
    fn a_missing_rotation_is_the_layers_turn_in_the_plane() {
        let (v, notes) = normalized(
            json!({"layers":[{"ty":4,"ks":{"rz":{"a":0,"k":45},"rx":{"a":0,"k":0}}},
                                                     {"ty":4,"ks":{}}]}),
        );
        assert_eq!(v["layers"][0]["ks"]["r"]["k"], json!(45));
        assert_eq!(v["layers"][1]["ks"]["r"]["k"], json!(0));
        assert_eq!(notes, vec!["3D rotation"]);
    }

    #[test]
    fn a_still_split_position_is_one_position_and_an_animated_one_is_refused() {
        let (v, _) = normalized(json!({"layers":[{"ty":4,"ks":{"r":{"a":0,"k":0}},"shapes":[
            {"ty":"tr","r":{"a":0,"k":0},"p":{"s":true,"x":{"a":0,"k":10},"y":{"a":0,"k":20}}}
        ]}]}));
        assert_eq!(
            v["layers"][0]["shapes"][0]["p"]["k"],
            json!([10.0, 20.0, 0])
        );
        let mut animated = json!({"layers":[{"ty":4,"ks":{"r":{"a":0,"k":0}},"shapes":[
            {"ty":"tr","r":{"a":0,"k":0},"p":{"s":true,"x":{"a":1,"k":[{"t":0,"s":[0]},{"t":9,"s":[5]}]},
             "y":{"a":0,"k":20}}}
        ]}]});
        assert!(normalize(&mut animated, &mut Vec::new()).is_err());
    }

    #[test]
    fn a_solid_layer_becomes_the_rectangle_it_is() {
        let (v, _) =
            normalized(json!({"layers":[{"ty":1,"sc":"#ff8000","sw":200,"sh":100,"ks":{}}]}));
        let layer = &v["layers"][0];
        assert_eq!(layer["ty"], json!(4));
        let items = &layer["shapes"][0]["it"];
        assert_eq!(items[0]["s"]["k"], json!([200.0, 100.0]));
        assert_eq!(items[0]["p"]["k"], json!([100.0, 50.0]));
        assert_eq!(items[1]["c"]["k"], json!([1.0, 128.0 / 255.0, 0.0, 1]));
        assert_eq!(
            layer["ks"]["p"]["k"],
            json!([0, 0, 0]),
            "no position: the origin"
        );
    }

    #[test]
    fn text_layers_unsupported_blends_and_empty_paths_go() {
        let (v, notes) = normalized(json!({"layers":[
            {"ty":5,"ks":{}},
            {"ty":4,"bm":16,"ks":{"r":{"a":0,"k":0}},"shapes":[
                {"ty":"sh","ks":{"a":0,"k":{"i":[],"o":[],"v":[],"c":false}}},
                {"ty":"rc"}
            ]}
        ]}));
        let layers = v["layers"].as_array().unwrap();
        assert_eq!(layers.len(), 1, "the text layer is gone");
        assert_eq!(layers[0]["bm"], json!(0));
        assert_eq!(
            layers[0]["shapes"].as_array().unwrap().len(),
            1,
            "the empty path is gone"
        );
        assert!(notes.contains(&"text") && notes.contains(&"blend mode add or hard mix"));
    }
}
