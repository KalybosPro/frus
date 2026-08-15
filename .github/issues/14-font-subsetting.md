title: Subset the bundled fonts at build time
labels: help wanted, performance, build

The bundled DejaVu faces are 3.4 MB, about 40% of a minimal application. They cover
far more of Unicode than any one application ever draws.

The four `bundled-*` cargo features are the current answer, and they are a coarse
instrument: they let you drop a whole face, not the 95% of a face you do not use.

### What to do

A build step that subsets the faces to the glyphs a build actually references —
`pyftsubset` does this, and there are Rust equivalents. This is where the next megabyte
is.

### The hard part, and why this is not a small job

Knowing which glyphs a build references. Static analysis of string literals gets you
the interface's own text and misses everything the application loads at runtime — a
task the user types, a name from an API. So a subset has to come with:

- a way for an application to declare the ranges it needs;
- a runtime fallback that degrades honestly when a glyph is missing, rather than
  drawing a blank box and leaving the developer to guess why;
- and it must never panic. Dropping a face is already a supported configuration
  (milestone 292) and the rule there holds here.

Worth agreeing on the interface in the issue before building the tool.
