# Milestone 360 — A list that counts from the bottom

The other half of a conversation view, and the half milestone 359 could not give: a
`Scroll` can anchor its content to the end, but only a **list** decides which end an
*index* is.

`List::reverse()` puts item 0 at the bottom, item 1 above it, and starts resting there.
With index 0 the newest message, appending one leaves every other item exactly where it
was — the view does not jump, and nothing has to be renumbered. That is the whole reason
the reference's `ListView` has a `reverse` at all.

## The window is the same arithmetic

A virtualised list computes which items to build from the offset. Reversing it does not
change that computation by a character:

```rust
let first = (offset_y / item_height).floor() as usize;
let last  = ((offset_y + viewport.height) / item_height).ceil() as usize;
```

Not a coincidence. A reversed list counts its **indices** from the end, and (since
milestone 359) a reversed scroll counts its **pixels** from the end — so index and offset
agree about which way forward is, and the window falls out unchanged. Only where an item
lands differs:

```rust
let top = if reverse {
    viewport.y + viewport.height - (i + 1) as f32 * item_height + offset_y
} else {
    viewport.y + i as f32 * item_height - offset_y
};
```

This is what milestone 359's choice of offset origin bought, and it is worth noting
because the alternative — numbering offsets from the top and reversing only the indices —
would have needed the window arithmetic mirrored as well, in a second place, disagreeing
with the first.

## Left

- **A reversed list still glows at the top** for movement refused at what is now its
  start. `edge_for` reads the sign of a refused delta in offset space, which a reversed
  axis has flipped. Nothing in the demo reverses yet, so it has not been seen; it is the
  same class as the pull-to-refresh note in milestone 359 and wants one fix covering both.
