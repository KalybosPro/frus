title: Clipboard on the web
labels: help wanted, web

Copy and paste work on desktop (`arboard`) and are simply absent on the web, so
`TextInput` on a web build silently drops Ctrl+C and Ctrl+V.

### What to do

Wire the async Clipboard API (`navigator.clipboard`) up behind the same interface the
desktop uses, so no widget knows which platform it is on.

### The interesting part

The web's clipboard is **asynchronous** and **permissioned**, and the desktop's is
neither. Reading may prompt the user, may be refused, and may only be allowed inside a
user gesture. Whatever shape you land on has to be one the desktop can satisfy without
pretending to be async, and one a widget can use without branching on the platform.

Worth agreeing on the interface in the issue before writing much code.

### Where

`crates/frus-shell` — see how the desktop clipboard is reached today, and
`crates/frus-widgets/src/text_input.rs` for the only caller that matters yet.
