# Milestone 554 — The way in is a component

## Objective

Milestones 551 to 553 gave the framework components, a text controller and a router. The
first things a person meets are still written the old way: the README's example, the
getting-started guide, the `cargo generate` template, `frus-hello`. A framework's
introduction *is* its model, so this step moves them, and adds the one thing `frus-hello`
needed that a component could not yet say: a timer.

## What changed

- **`use_interval`.** A component asks for a timer with
  `cx.use_interval(period, cx.callback(|state| …))`. The shell diffs what the latest build
  asked for, as it diffs an application's subscriptions: a timer nobody asks for any more is
  stopped, one that keeps being asked for keeps running, and two timers of the same period are
  told apart by the component that asked and the order it asked in.
  `Subscription::every_keyed` is the piece underneath.
- **`frus-hello` and `templates/app`** are `StatefulWidget`s: a state struct with plain
  methods, handlers that are closures, and — in `frus-hello` — automatic counting through
  `use_interval`. Tests are now of the state (plain Rust) and of the widget built headless.
- **Documentation:** the README (English and French), the getting-started guide, ARCHITECTURE,
  and the two crate READMEs whose example is compiled.

## Decisions

**A timer is asked for, not started.** The alternative was an effect that spawns a thread and
delivers messages through a channel the shell would have to expose to components. The
subscription machinery already does exactly that — start, keep, stop, on every platform,
including `setInterval` on the web — so a timer is a *declaration* the same machinery reads.
The cost is that `use_interval` is not a slot-based hook: it may be called conditionally,
which is the point (`if self.auto { … }`) and is documented as the one exception to the order
rule.

**The shell diffs after a build, not only after a message.** A timer appears in the build a
message caused, which is later than the message. Diffing only after messages would have
started it on the *next* one. `sync_subscriptions` now also runs after each rebuild; for an
application on the typed-message model it is one more call of a function it already calls on
every message.

**The template had drifted, and this found it.** Its `Variant::Primary` and `Variant::Secondary`
were renamed to `Filled` and `Outlined` long ago; the template does not compile in the
workspace, so nothing noticed. The README carried the same names. The template's source is now
compiled once in a scratch crate as part of this change, which is what should have been
happening.

**`frus-demo` stays on the typed-message model.** It is a 2,500-line application with its own
tests written against messages, and it is a good example of when that model is worth choosing.
Moving it is a decision about the demo, not a consequence of this step.

## Verification

- Two tests on the interval: a build asks for its timers and asks again identically, and one
  that stops asking stops the timer; and through the shell, a timer is a subscription for
  exactly as long as a component asks. One on `every_keyed`.
- `frus-hello`'s two tests, and the template source compiled and tested in a scratch crate.
- The whole workspace: check, clippy with `-D warnings`, tests, doctests, rustdoc with
  `-D warnings`.

## Left out

`frus-demo`, `frus-fetch-example` (asynchronous work in the component model: a future a component
starts and a `set_state` when it completes) and `frus-transforms` remain on the typed-message
model.
