title: Accessibility on the web
labels: help wanted, web, accessibility

The semantics tree already exists and is populated — every widget contributes to it,
and the desktop shell hands it to AccessKit, where a screen reader reads it. On the
web that tree is built and then dropped.

AccessKit has web support. This is a bridging job, not a from-scratch one, which is
what makes it a good way into the project: the hard part (deciding what a widget
*means*) is already done.

### What to do

1. Read how `crates/frus-shell/src/a11y.rs` hands the tree to AccessKit on desktop.
2. Do the same through AccessKit's web adapter, which projects into the DOM.
3. Check it with a real screen reader. Not with the accessibility inspector — with a
   screen reader, using the demo, with your eyes shut for at least part of it.

### Done when

The demo's task list can be navigated and operated on the web without sight: the
buttons announce what they do, the checkboxes announce their state, and a task added
is announced.
