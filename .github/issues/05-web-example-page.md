title: A real page for the web example
labels: good first issue, documentation, web

`crates/frus-hello/web/index.html` is the bare minimum that loads the wasm module. It
is the first thing anyone sees when they try frus in a browser, and it looks like a
test fixture, because that is what it is.

### What to do

- A page that does not look abandoned: a title, a line saying what is being shown, a
  link back to the repository.
- Say what is required — a WebGPU-capable browser on a secure context — **before** the
  canvas fails, and detect it rather than leaving a blank rectangle when `navigator.gpu`
  is missing.
- A visible loading state while the wasm module is fetched. It is a few hundred
  kilobytes and on a slow connection the page is currently blank for it.

### Notes

Keep it a single static HTML file with no build step. The point of the example is that
`python3 -m http.server` is enough to run it.
