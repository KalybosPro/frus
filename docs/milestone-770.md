# Milestone 770 — A field you can see above a landscape keyboard on the Web

## Objective

Milestone 765 ran the Web build in Chrome on a real phone. In landscape, the keyboard left the page
38 CSS pixels. 765 stopped the keyboard from going up and down, but the field being typed into
stayed out of sight: the person typed into a field they could not see. This milestone makes that
field visible.

## What the phone reports

Huawei STK-L21, Chrome, SwiftKey, landscape (`innerWidth` 872, `innerHeight` 339):

| | keyboard down | keyboard up |
|---|---|---|
| layout viewport (`innerHeight`) | 339 | 339 (the page keeps its size) |
| visual viewport (`visualViewport.height`) | 339.2 | 38.4 |
| frus canvas | 36.18 to 339.2 (under the demo page's header) | the same |

Chrome lays the keyboard over the page and shrinks only the visual viewport. On a plain page
with an `<input>` at y = 250, focusing the input shows the keyboard, and Chrome then **pans the
visual viewport** to the field: `visualViewport.offsetTop` went from 0 to 252 in a few
animation steps, and the field was in the 38-pixel strip. That is what a web page does, and what
a phone's own app does when its window cannot be shrunk to what the keyboard leaves (it pans
the window).

frus did not do that. Since milestone 749 the shell reports the keyboard as an inset, so the
application lays itself out above the keyboard and scrolls the field into the remaining space.
That is right in portrait (the keyboard leaves about 286 pixels of canvas). In landscape the
inset left the canvas 1.8 pixels: the demo's app bar alone is 56. The scaffold's body got no
height, the field was not laid out, and since milestone 749 the editing element is kept in the
part the keyboard leaves, so the browser had nothing to pan to.

The other toolkit's web engine does the same as frus did: the keyboard becomes the view's
bottom inset (physical height minus the visual viewport's height), so a scaffold that resizes
for the keyboard gives its body no height in the same case.

## The change

The page now chooses, per keyboard, how it makes room for it (`web_page::KeyboardFit`):

- **Resize** (as before): the keyboard is an inset, and the application lays itself out above
  it.
- **Pan**: the application keeps its whole size, the editing element sits exactly over the
  field, and the page is panned so the field is in what the keyboard leaves
  (`scrollIntoView({block: "center"})` on the element, when it is not already in sight).

The choice is made from a frame laid out above the keyboard (`KeyboardFitter::learn`, called
after each frame by `App::fit_web_keyboard`). If that frame did not lay out the focused field,
or what the keyboard leaves of the canvas is shorter than the field, the field cannot be seen
above the keyboard however the application scrolls, so the next frame pans
(`keyboard_fit`). The choice is kept:

- a pan for as long as the keyboard keeps its height. Moving to the next field does not lay the
  page out above the keyboard again.
- a resize for the field it was made for, so a field that is later scrolled out of a lazily
  built list does not turn the keyboard into a pan.

A new keyboard height (a rotation, a keyboard that changes mode), a new field under a resize,
or the keyboard going away starts over. The keyboard is read as what the visual viewport leaves
out of the layout viewport (`keyboard_view`), which a pan does not change.

While the page pans, `App::compute_insets` reports no keyboard cover, and
`web_editing::element_top` puts the element over the field instead of keeping it out of the
keyboard's way.

## How it was tested

Unit tests (`frus-shell`), with the values measured on the phone:

- `web_page::tests::a_field_with_no_room_above_the_keyboard_pans_the_page`
- `web_page::tests::the_fit_is_chosen_once_per_keyboard_and_field`
- `web_page::tests::a_keyboard_is_a_visual_viewport_shorter_than_the_page`
- `web_page::tests::the_room_is_what_the_visual_viewport_shows_of_the_canvas`
- `web_page::tests::a_field_is_in_sight_when_shown_as_much_as_it_can_be`
- `web_editing::tests::the_element_is_over_the_field_when_the_page_pans`: with the old
  `element_top` (no pan), the element over a field at 300 was kept at 111.9, in the top quarter of
  the canvas, and the browser had no reason to pan to the field.

PHONE_RESULTS

## Not done

NOT_DONE
