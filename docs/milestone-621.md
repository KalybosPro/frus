# Milestone 621 — An iOS page let go

## Objective

Milestone 620 gave the iOS slide its look; the swipe that drags it back still ended the frus
way. Let go, a page went or stayed according to where the finger's momentum would have
carried it 120 ms later, and finished on a spring. The reference's iOS route decides
otherwise (`route.dart:851`, `dragEnd`), and finishes otherwise.

## What was done

Under a theme whose transition is the iOS slide, a back swipe let go:

- **goes or stays by its direction** when it is moving at a screen width a second or more
  (`_kMinFlingVelocity`, `route.dart:32`): towards the edge it was being dragged to, it
  goes, even from near the start; the other way, it stays, even from near the end;
- **goes or stays by how far it got** when it is slower: past halfway it goes;
- **finishes in 350 ms** (`_kDroppedSwipePageAnimationDuration`, `:35`) on
  `fastEaseInToSlowEaseOut`, from wherever it was, still drawn linearly under the gesture
  until it has finished, so the pages do not change curve mid-flight (`:899`).

Under the other transitions the swipe is frus's own — the reference has none there — and
keeps its rule and its spring.

## Not done

The shell plays the system's back key (Android's back button, the browser's) as a swipe let
go at once, so under an iOS theme it now settles in 350 ms instead of the 500 ms pop the
reference plays for a button. A pop that is not a gesture should be a plain transition; that
belongs to the shell, and to the predictive back still missing on Android.

## Tests

- `an_ios_page_let_go_follows_the_ios_rules`: a flick back pops from near the start, a flick
  the other way keeps the page from near the end, slow releases are decided by halfway, and
  the settle lasts 350 ms from wherever the page was.
- `a_back_gesture_past_halfway_pops_and_a_short_one_does_not` still holds the frus rule
  elsewhere.

Mutation testing: four mutants, all killed — the fling threshold doubled, the direction
turned round, halfway moved to 30 %, and the settle lengthened to 500 ms.
