use frus::{Callback, CircleAvatar, Color, ListTile, Text, TextOverflow, LIST_TILE_HEIGHTS};

use crate::models::post::Post;

/// The height of one row. The list is virtualised — it builds only the rows on screen —
/// which it can do because it knows where row `n` starts: every row is this tall, the
/// height of a three-line tile.
pub(crate) const POST_TILE_HEIGHT: f32 = LIST_TILE_HEIGHTS[2];

/// One post in the list: its author's id, its title, the start of its body.
///
/// A function returning a plain widget, not a `StatelessWidget`: a virtualised list
/// builds its rows while the frame is laid out, after the framework has built the
/// components of the tree, so a row must be a widget that needs no building — a
/// component there is never built and stops the app.
pub(crate) fn post_tile(post: &Post, muted: Color, on_tap: Callback) -> ListTile {
    // The body comes with line breaks; a subtitle reads better as one paragraph. A
    // three-line tile has room for two lines of it, and the tile does not cut the text
    // for it: the excerpt says so itself, or a long body overflows the row.
    let excerpt = Text::new(post.body.replace('\n', " "))
        .size(14.0)
        .color(muted)
        .max_lines(2)
        .overflow(TextOverflow::Ellipsis);
    ListTile::new()
        .leading(CircleAvatar::new().child(Text::new(post.user_id.to_string())))
        .title(capitalize(&post.title))
        .subtitle_child(excerpt)
        .three_line()
        .on_tap(on_tap)
}

/// The placeholder API writes its titles in lower case.
pub(crate) fn capitalize(text: &str) -> String {
    let mut chars = text.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capitalize_touches_only_the_first_letter() {
        assert_eq!(capitalize("sunt aut facere"), "Sunt aut facere");
        assert_eq!(capitalize(""), "");
        assert_eq!(capitalize("éclair"), "Éclair");
    }
}
