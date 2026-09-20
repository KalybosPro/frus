//! **Links**: an address that comes from outside the application and names a place in it.
//!
//! `https://example.com/orders/42?tab=items` (an Android app link), `myapp://orders/42` (a
//! custom scheme, on Android or on a desktop's command line) — each is one string, and the
//! application understands *locations*: `/orders/42?tab=items`. This is the whole of the
//! translation, so that the platforms hand the application the same thing the web's address
//! bar does, and [`Application::open_location`](crate::Application::open_location) is the one
//! way in.
//!
//! The rule is short and has one choice in it. **A web link's path is the location.** A
//! **custom scheme has no host in the usual sense** — `myapp://orders/42` means "orders/42" to
//! whoever wrote it, not a server called `orders` — so there the host is the first segment.

/// The location a link stands for, or `None` if `link` is not one: no scheme, or nothing after
/// it.
///
/// | link | location |
/// |---|---|
/// | `https://example.com/orders/42?tab=items` | `/orders/42?tab=items` |
/// | `https://example.com` | `/` |
/// | `myapp://orders/42` | `/orders/42` |
/// | `myapp:orders/42` | `/orders/42` |
/// | `myapp:///orders/42` | `/orders/42` |
///
/// A fragment is dropped: an application's locations have none.
pub fn location_of_link(link: &str) -> Option<String> {
    let link = link.trim();
    let (scheme, rest) = link.split_once(':')?;
    let scheme_ok = !scheme.is_empty()
        && scheme.starts_with(|c: char| c.is_ascii_alphabetic())
        && scheme
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '-' | '.'));
    if !scheme_ok {
        return None;
    }
    let rest = rest.split('#').next().unwrap_or("");
    let web = scheme.eq_ignore_ascii_case("http") || scheme.eq_ignore_ascii_case("https");
    let location = match rest.strip_prefix("//") {
        Some(after) if web => match after.find(['/', '?']) {
            // The authority is the server's, and is dropped.
            Some(at) => after[at..].to_string(),
            None => String::new(),
        },
        // A custom scheme's "host" is the first segment of the place.
        Some(after) => after.to_string(),
        None => rest.to_string(),
    };
    Some(normalised(&location))
}

/// `location` as a location: one leading slash, and `/` for nothing.
fn normalised(location: &str) -> String {
    let trimmed = location.trim_start_matches('/');
    if trimmed.is_empty() {
        "/".to_string()
    } else {
        format!("/{trimmed}")
    }
}

/// The link among a program's arguments, if one was passed: the first that has the shape of
/// one. What the operating system does when a registered scheme is opened is to start the
/// program with the link as an argument.
pub fn link_among<I, S>(args: I) -> Option<String>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    args.into_iter()
        .map(|arg| arg.as_ref().to_string())
        .find(|arg| arg.contains("://") && location_of_link(arg).is_some())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_web_links_path_and_query_are_the_location() {
        assert_eq!(
            location_of_link("https://example.com/orders/42?tab=items"),
            Some("/orders/42?tab=items".to_string())
        );
        assert_eq!(
            location_of_link("http://localhost:8080/a"),
            Some("/a".to_string()),
            "a port belongs to the authority"
        );
    }

    #[test]
    fn a_web_link_with_no_path_is_the_root() {
        assert_eq!(
            location_of_link("https://example.com"),
            Some("/".to_string())
        );
        assert_eq!(
            location_of_link("https://example.com/"),
            Some("/".to_string())
        );
        assert_eq!(
            location_of_link("https://example.com?ref=mail"),
            Some("/?ref=mail".to_string())
        );
    }

    #[test]
    fn a_custom_schemes_host_is_the_first_segment() {
        assert_eq!(
            location_of_link("myapp://orders/42"),
            Some("/orders/42".to_string())
        );
        assert_eq!(
            location_of_link("myapp://settings"),
            Some("/settings".to_string())
        );
    }

    #[test]
    fn a_custom_link_without_slashes_or_with_three_names_the_same_place() {
        assert_eq!(
            location_of_link("myapp:orders/42"),
            Some("/orders/42".to_string())
        );
        assert_eq!(
            location_of_link("myapp:///orders/42"),
            Some("/orders/42".to_string())
        );
        assert_eq!(location_of_link("myapp://"), Some("/".to_string()));
    }

    #[test]
    fn the_fragment_is_dropped() {
        assert_eq!(
            location_of_link("https://example.com/a?x=1#top"),
            Some("/a?x=1".to_string())
        );
    }

    #[test]
    fn what_is_not_a_link_is_none() {
        assert_eq!(location_of_link(""), None);
        assert_eq!(location_of_link("/orders/42"), None, "no scheme");
        assert_eq!(
            location_of_link("42:orders"),
            None,
            "a scheme starts with a letter"
        );
        assert_eq!(location_of_link("not a link"), None);
    }

    #[test]
    fn the_link_among_a_programs_arguments_is_the_first_that_looks_like_one() {
        assert_eq!(
            link_among(["frus-demo", "--verbose", "myapp://orders/42", "other://x"]),
            Some("myapp://orders/42".to_string())
        );
        assert_eq!(link_among(["frus-demo", "--verbose"]), None);
        assert_eq!(link_among(["frus-demo", "C:\\Users\\me"]), None);
    }
}
