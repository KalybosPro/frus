# Milestone 676 — Graphs in the performance panel

## Objective

`P` (milestone 668) showed what the last frame cost, as text. A number says how one frame went; a
stutter is a pattern over many, and the reference's overlay shows it as graphs: one for the
interface's work, one for the drawing's, against the budget of a frame.

## What was done

- The panel keeps the costs of the last 90 frames: the interface (building, laying out and
  painting the tree) and the drawing (encoding, submitting, waiting for the GPU and the display).
- Under the figures, two graphs, one bar a frame, the newest on the right. A white line marks the
  budget of a 60 Hz frame, 16.7 ms, at mid-height; a bar over it is red. A frame over twice the
  budget fills the graph.
- Turning `P` off and on starts the history again.

## Verified

- On the desktop, the counter example counting by itself: the interface graph stays low and
  green; the drawing graph is red, over the line — about 29 ms a frame on this machine's
  integrated GPU, which is what the figures above it say too.
- Unit test: the history keeps the latest 90 frames, the newest last; a frame within budget is
  green, one over the top fills the graph and is red, the budget itself at half height.
