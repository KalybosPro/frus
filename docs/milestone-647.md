# Milestone 647 — `CircleAvatar`, as the reference's

## Objective

The next widget in the review against the reference (tracker `docs/reference-review.md`),
after the decoration it is built on (641, 644-646). frus's `CircleAvatar` was a pill of
initials. The reference's is a circle standing for a user, with whatever it is given
inside it (`material/circle_avatar.dart`):

| | reference | frus before |
|---|---|---|
| content | any child, centred | two initials, nothing else |
| its type | the theme's `titleMedium`, in the foreground colour; icons the same colour; no text scaling | two fifths of the diameter |
| colours | `primaryContainer`, `onPrimaryContainer` | `primary`, `on_primary` |
| pictures | `backgroundImage` and `foregroundImage`, each `BoxFit.cover` in the circle | none |
| size | `radius`, or `minRadius` and `maxRadius`; 20 by default | a diameter, 36 by default |
| changes | colour and size animated over `kThemeChangeDuration` | none |

## What was done

- **`CircleAvatar<Msg>`**: `new()` and `child(widget)`, and `initials(name)` for the common
  case. The child is centred, and the avatar is sized as the reference's `AnimatedContainer`
  under its constraints:
  - `radius` fixes both bounds;
  - `min_radius` alone leaves the largest unbounded, `max_radius` alone the smallest at
    nought;
  - nothing at all is a radius of 20.
  `size(diameter)` stays, as a radius of half of it.
- **The child's type and icons**, through the hooks a subtree's theme and media pass
  through:
  - the theme's `title_medium` in the foreground colour, for text;
  - the same colour for icons;
  - a text scale of one, so the initials do not spill out of a circle that did not grow.
- **Colours**: the scheme's `primary_container` and `on_primary_container`, or those given.
- **Pictures**: a background picture covering the circle under the child, a foreground one
  over it, each the J645 `DecorationImage` with `BoxFit::Cover` in a circular decoration. A
  picture still on its way is `None` and shows what is under it: the foreground falls back
  to the background, and that to the colour, as the reference's do when a picture fails.
- **Animation**: a colour given and a fixed radius given change over 200 ms, through the
  runtime's animated colour and size.
- **`Image::handle`**: the decoded pixels of an `Image::memory` or `Image::network`, so the
  same loading and cache feed an avatar or a decoration.

**Changed**:

- `CircleAvatar::new(name)` is `CircleAvatar::initials(name)`; `new()` is an empty avatar.
- `color` is `background_color`.
- The defaults moved to the reference's: 40 across, the container colours, initials in
  the title's size.
- `CircleAvatar` takes its child's message type.

Four pictures moved with it, each checked: the drawer header's three avatars, whose dark
initials on a dark green can now be read, and three tables' avatars.

## Decisions

- **The theme's own colour is not animated.** The reference animates whatever colour it
  ends up with, a theme change included; here only a colour given is, since the animated
  value is read before the theme is.
- **No error callbacks.** `onBackgroundImageError` and `onForegroundImageError` report a
  failed load; `Image::error` answers the same question where the picture was made.

## Tests

- `frus-widgets`:
  - initials of one word, two, three, and none;
  - the reference's sizes for each combination of bounds, and the centring;
  - the container colours, and a colour given;
  - the child's type and icons in the foreground colour, a colour given, no text scaling,
    and none of it with no child;
  - the pictures covering the circle, a foreground one over the child, and a picture on its
    way showing nothing;
  - a given colour and radius animated over 200 ms, and the theme's not; the colour in
    flight is the one painted;
  - a selection spanning the texts reaching the initials;
  - `Image::handle`: the pixels once here, none while loading or failed.
- `frus-test`: a new picture, `avatars`, checked by eye:
  - top: initials, an icon, initials on a colour given, two letters in a large circle;
  - bottom: a background picture, a foreground picture over initials, a foreground picture
    still on its way over a background one, and an avatar between two radii as big as its
    child.

Mutation testing: 17 mutants, all killed. They cover each bound of the size and the
fixed one, the centring, both default colours, the colour in flight, the background
picture and its fit, the child's theme and icons, the selection kept, the text scale, the
animated radius and duration, and `Image::handle`.
