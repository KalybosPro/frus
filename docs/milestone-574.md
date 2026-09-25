# Milestone 574 — The task screen's words sit under their avatar

Seen on the phone while checking milestone 573: on the demo's task screen the avatar is in the
middle of the screen and the title and state label under it are not — they sit a quarter of the way
across, both centred on a point 97 px from the left of a 411 px screen.

## Cause

Not the words, and not the column that centres them: the layout was dumped level by level.

| widget | box |
|---|---|
| the `Container` that holds the words | x 24, **363 wide** — the whole width |
| `FractionalTranslation` (the slide) | x 24, **147 wide** |
| `Opacity` (the fade) | x 24, 147 wide |
| the `Flex` of the two words | x 24, 147 wide |

The slide and the fade wrap their child in boxes that take the child's **own** width, and the
container gave that shrunk box the start of its own. The words were centred, inside a box exactly
as wide as the widest of them, at the left edge. Nothing about the screen's width could reach them
through two auto-sized wrappers, and a `width_fraction(1.0)` on the column — tried first — is a
percentage of an auto-sized parent and so of nothing.

## The change

The container that holds the transitions now **aligns its child to the centre** (`alignment(
Alignment::CENTER)`), which is where the box the transitions produce belongs. A title that is long
enough to wrap still has the container's full width to wrap in: the milestone-289 test for that
passes unchanged. A test for this milestone states the symptom — the title's and the state label's
middles are the screen's middle — and failed at 97.5 against 205.5 before the change.

On the phone (release APK of the demo), the title and the state label are centred under the
avatar, on the screen's middle.

## Left

- **The wrappers themselves.** A `SlideTransition` or a `FadeTransition` around a child that asks to
  fill its parent does not pass the request on: the wrapper is as wide as the child's content. A
  transition is meant to be invisible to layout, and this is where it is not. Passing the child's
  sizing through the wrappers touches every use of them and the goldens, and is a change of its own;
  this milestone fixes the screen that showed it and not the wrappers.
- `frus-demo` gained `frus-text` as a dev-dependency, to measure a word in the test: a text
  primitive is a point, and the middle of a word needs its width.
