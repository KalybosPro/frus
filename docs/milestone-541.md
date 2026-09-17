# Milestone 541 — `frus-bench` denies a missing doc comment

Issue #14, continuing the sweep. `frus-bench` is the smallest crate left: one file,
`VIEWPORT` and four functions (`task_list`, `task_list_wordless`, `nested`, `build`)
that build the trees every benchmark measures, so a number from one run means the
same thing as a number from the next.

`#![warn(missing_docs)]` found nothing — every public item was already documented,
each with what it is *for*, not a restatement of its name (`task_list_wordless`
says what the gap between it and `task_list` measures, `nested` says why depth
matters separately from drawn output). Straight to `#![deny(missing_docs)]`.
