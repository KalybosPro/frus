# Milestone 620 — Page transitions, platform by platform

## Objective

Every page in frus arrived the same way: sliding in from the right while the page under it
drifted 30 % of the way out under a dimming, on a spring. The reference's pages arrive the
way the platform's own do. Its `PageTransitionsTheme` (`page_transitions_theme.dart:764`)
maps each platform to a builder: Android fades forwards, iOS and macOS slide, Linux and
Windows zoom — and a platform with no entry zooms. It reads the **theme's** platform, so a
theme set to iOS slides on any device. Each builder runs for its own duration at a constant
pace, and shapes that pace with its own curves (milestone 619).

## What was done

- **`PageTransitionsBuilder`**: `Zoom`, `Cupertino`, `FadeForwards`, each with its
  `transition_duration()` — 300, 500 and 450 ms — and a pure `frame(progress, forward,
  gesture)` saying how the page on top and the page under it look at that moment, and
  what lies between them. The numbers are the reference's:

  | | push | pop |
  |---|---|---|
  | **Zoom** (`:116-298`) | the page from 85 % to 100 % on the emphasized curve, fading in over 12.5–25 %, over a surface scrim rising to 60 % over 20.75–41.75 %; the page under it to 105 % | the page to 90 %, fading out over 8.25–20.75 %; the page under it settling from 110 % |
  | **Cupertino** (`route.dart:531-572`) | the page in from the end edge on `fastEaseInToSlowEaseOut`; the page under it a third of the way out on `linearToEaseOut`, under a black barrier easing in to `0x18`; a faint shadow along the page's start edge | the mirror curves; `easeInToLinear` for the page under it |
  | **FadeForwards** (`:396-552`) | the page a quarter of the way in on the emphasized curve, fading in over the first 75 %; the page under it a quarter of the way out, fading out over the first 25 %, the surface behind both | the same, reversed |

  The Cupertino slide turns round under a right-to-left layout; the fade does not, as the
  reference's does not.
- **`PageTransitionsTheme`**, on `Theme::page_transitions`, with the reference's defaults,
  `with(platform, builder)` to change one, and `builder_for(platform)` with the reference's
  fallback.
- **The navigator** paints the builder's frame for the theme's platform: the pages moved,
  scaled about their centres and faded, each in one layer only when it is not at rest, with
  what can be touched in it moving with it; the backdrop, barrier, scrim and edge shadow in
  between.
- **The router** runs a transition at a constant pace over the theme's builder's duration
  instead of a spring, so the builder's curves are the only shaping, as in the reference.
  A back gesture marks its navigator as driven by a finger (`Navigator::gesture`), and the
  pages follow it linearly, as the reference's do while a pop gesture is under way.
- **Shared elements fly above both pages.** A page in a transition may now be inside a layer
  that zooms or fades it; the flight takes its hero out of whichever layer holds it and
  draws it above both, untouched by either, as the reference's hero flies in the overlay.
  A page at nothing at all — the zoom's first frame — is still a layer, so that the flight
  can take its hero from it.

## Not done

- **Android's predictive back.** The reference's Android default, `PredictiveBackPageTransitionsBuilder`,
  fades forwards when no back gesture is under way, which is what frus does; during the
  system's predictive back gesture it shrinks the page as the finger moves. frus does not
  receive that gesture yet.
- **The iOS swipe's release rules** — a fling of a screen width per second commits, a 350 ms
  settle on `fastEaseInToSlowEaseOut` — are the next milestone; the settle is still the
  spring.
- Letter spacing and Apple's own bars, as milestone 618 says.

## Tests

- `each_platform_has_the_reference_s_transition`, `every_transition_begins_and_ends_at_rest`,
  `the_zoom_pushes_from_85_percent_over_a_scrim`, `the_zoom_pops_by_shrinking_away`,
  `the_cupertino_slide_parallaxes_a_third`, `the_fade_forwards_moves_a_quarter`: the tables,
  number by number.
- The navigator's tests now say which transition they describe — the slide, under a theme
  that follows iOS — where they followed whatever machine ran them. The hero tests run
  under a theme that follows Linux, whose zoom puts the pages in layers: the hard case.

Mutation testing: eight mutants, all killed — Android given the zoom, the zoom starting from
90 %, the slide's parallax a quarter instead of a third, the fade's fade-in over half the
way, every push timed as the zoom's, a back gesture not marked as the finger's, a page's
layer not scaled, and a hero not looked for inside the pages' layers. The fade-in first
survived: its test checked the ends and not the pace; it now checks the halfway point.
Three more tests came with them: a push's duration on iOS and Linux, a back gesture drawn
under the finger, and a push under a desktop theme scaling each page in its layer. The
navigator's golden is now three, one per transition, each under its platform.
