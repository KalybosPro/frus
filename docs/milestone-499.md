# Milestone 499 — The second deceleration profile, and what chooses it

One of the two loose ends of [#55](https://github.com/KalybosPro/frus/issues/55). The
stretch overscroll is untouched and stays open, for the reason the issue itself gives.

## The issue's premise is wrong, and the correction is the milestone

> The bouncing physics implement one deceleration profile. The reference has two, and
> switches between them **by velocity**.

The reference does have two. It does not switch by velocity, and it never has. It switches
by **what is doing the scrolling** — and the choice is made once, when the physics are
built, not per fling:

> `normal` — Standard deceleration, aligned with mobile software expectations.
> `fast` — Increased deceleration, aligned with desktop software expectations. Appropriate
> for use with input devices more precise than touch screens, such as trackpads or mouse
> wheels.

A finger *throws* a surface and lets go of it. A trackpad or a wheel is a hand resting on a
device that reports motion. Those two want different endings, and no amount of velocity
turns one into the other — a slow trackpad gesture is still a trackpad gesture. So the
profile is a property of the physics a scrollable is built with, resolved from the platform,
which is what this milestone implements.

Had it been done as asked — switched by speed — a hard fling and a gentle one on the same
device would have ended in different ways, and the seam would have been at whatever
threshold was picked. There is a test that says this out loud: the same physics, asked at
two speeds, must produce the same *kind* of ending.

## And it is four differences, not a constant

The other half of the correction. "The second profile" reads like a number to swap; it is
four separate things, and only three of them are numbers:

| | normal (a finger) | fast (a trackpad, a wheel) |
|---|---|---|
| ballistic deceleration | drag alone | drag **plus a constant**, 1400 px·s⁻² |
| overscroll band, at rest | 0.52 | 0.26 — twice as stiff |
| fling velocity cap | 8 000 px/s | ×8 |
| easing back out of an overscroll | resisted | **not resisted at all** |

The last one is a behaviour rather than a value, and it is the one a reader notices. A
finger that pulled a list past its end is still holding the band, and letting it back
gradually is what tension feels like. A trackpad is holding nothing, and the same stickiness
reads as the content refusing to come back.

## Why the constant term needed Newton's method

Drag alone is geometric: `dx(t) = v₀·Dᵗ`. The velocity is multiplied down each second and
reaches nought only at infinity — exactly right for a surface that was thrown, and it is
also what makes the closed forms work. The stopping time does not exist, and the instant the
motion passes a given point inverts to a logarithm.

Add `− c·t` and both go. There is no closed form for when `v₀·Dᵗ − c·t = 0`, and none for
when `x(t)` reaches a given position. Both are solved by **ten steps of Newton's method** —
ten and not "until it converges", because a fixed count cannot hang and this runs while a
finger is still on the glass. That is the reference's own iteration count, and it is the
reason `FrictionSimulation` grew a `final_time` at all: with a constant term the motion has
an end, and everything after it must report the resting place rather than continuing to
integrate past it.

The crossing time is the one that would fail quietly. It is what hands a fling over to the
edge spring at exactly the right instant, so a wrong answer there starts a bounce early or
late and nothing else in the framework complains.

## Nought changes nothing

The new term is added so that the existing motion is **bit for bit** what it was: `new` is
`with_constant_deceleration(…, 0.0)`, and there is a test comparing the two at six instants
with `assert_eq!` rather than a tolerance. Every hand-held build of this framework runs the
first profile, and a milestone that adds a second answer may not perturb the first.

## What this does not have: a hand

The issue is explicit, and it is right:

> Both should be verified **on a device**, not only in a test. The glow's first version
> passed every test it had and was wrong in the hand, which is how milestone 301 came to
> exist.

**The fast profile has not been felt, and this milestone does not claim it has.** It is the
desktop-bouncing profile: the platform default picks it on macOS and nowhere else, and the
machines this repo is built and tested on are Windows, Android and Linux, where the default
is clamping. So what is verified here is the arithmetic — that the motion stops, stops short
of where coasting would carry it, stops in both directions, and that the band and the cap
are what they should be — and *not* that a trackpad flick feels right under a hand.

Saying so is the point. Milestone 301 exists because a green suite was taken for a verdict
about a feeling. Whoever next has a Mac in front of them should scroll a long list with a
trackpad and say whether the ending reads as an ending; until then this is a correct
implementation of a specified profile, which is a different claim.

## Still open on #55

**The stretch overscroll effect**, untouched. It is not a parameter: a glow is a primitive
drawn over the content, a stretch is a deformation *of* it, so it needs the subtree rendered
to a texture and sampled with a displacement. The issue names the place to start reading —
the separable blur pre-pass built for `ImageFiltered` in milestone 339 — and that is still
where it starts.
