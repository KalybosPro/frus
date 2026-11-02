#!/usr/bin/env python3
"""Checks, in a real browser, that a frus field in a frame lets the page around it have the focus
(milestone 734).

While a field has the focus, the shell keeps the browser's focus on its editing element, and takes
it back when it finds it nowhere in particular. From inside a frame, the page around it taking the
focus looks just like that: the frame's active element falls back to its body. So the field took
the focus back within a frame, and nothing else on the page could be typed in. The check puts the
demo in a frame beside an `<input>` of the page's, and:

- focuses the demo's add field: the keyboard is in the frame;
- presses the page's input: it has the focus, keeps it, and takes what is typed;
- presses the field again: the keyboard is back in the frame, and what is typed reaches the field.

    pip install websocket-client
    # build the demo for the web and serve it (crates/frus-demo/web/README.md), then:
    python3 scripts/web-frame-focus-check.py http://127.0.0.1:8080/

Needs a browser with WebGPU. Set BROWSER to its executable if it is not found.
"""
import importlib.util, os, subprocess, sys, tempfile, time, urllib.request

HERE = os.path.dirname(os.path.abspath(__file__))
spec = importlib.util.spec_from_file_location("a11y", os.path.join(HERE, "web-accessibility-check.py"))
a11y = importlib.util.module_from_spec(spec)
spec.loader.exec_module(a11y)

failures = []


def expect(label, got, want):
    ok = got == want
    print(f"  {'ok  ' if ok else 'FAIL'} {label}: {got!r}" + ("" if ok else f"   (wanted {want!r})"))
    if not ok:
        failures.append(label)


# An address of the same origin with no page of its own, so that the page here can read the frame.
HOST = """(() => {
  document.body.innerHTML = '<input id="own" style="display:block;width:300px;height:40px">'
    + '<iframe id="demo" style="display:block;width:900px;height:680px;border:0"></iframe>';
  document.getElementById('demo').src = %r;
})()"""
FRAME = "document.getElementById('demo')"
INNER = f"{FRAME}.contentDocument"
FIELD = f"{INNER}.querySelector('[data-frus-field]')"
OUTER_ACTIVE = "document.activeElement.id"
INNER_ACTIVE = f"{INNER}.activeElement.tagName"


def centre(p, inner_rect):
    return p.js(f"(() => {{ const r = {inner_rect}.getBoundingClientRect(); "
                f"return [r.x + r.width / 2, r.y + r.height / 2]; }})()")


def main(base):
    profile = tempfile.mkdtemp(prefix="frus-frame-focus-check-")
    browser = subprocess.Popen([
        a11y.find_browser(), "--headless=new", f"--remote-debugging-port={a11y.PORT}", f"--user-data-dir={profile}",
        "--no-first-run", "--enable-unsafe-webgpu", "--enable-features=Vulkan,WebGPU", "--ignore-gpu-blocklist",
        "--force-renderer-accessibility", "--window-size=1000,800", *a11y.FOREGROUND, "about:blank",
    ], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    try:
        for _ in range(50):
            try:
                urllib.request.urlopen(f"http://127.0.0.1:{a11y.PORT}/json", timeout=2)
                break
            except Exception:
                time.sleep(0.2)
        p = a11y.open_at(base.rstrip("/") + "/no-such-page-frame-focus-check")
        p.wait(1)
        p.js(HOST % (base.rstrip("/") + "/index.html"))
        p.wait(8)

        print("the field in the frame")
        node = f"{INNER}.querySelector('canvas [role=textbox][aria-label=\"New task\"]')"
        if not p.js(f"!!{node}"):
            print("  FAIL no field named 'New task' in the frame")
            return 1
        p.js(f"{node}.focus()")
        p.wait(0.8)
        expect("the frame has the page's focus", p.js(OUTER_ACTIVE), "demo")
        expect("its input has the frame's", p.js(INNER_ACTIVE), "INPUT")
        p.call("Input.insertText", text="first")
        p.wait(0.5)
        expect("the field takes what is typed", p.js(f"{FIELD}.value"), "first")
        # Where the field is, while it is there to measure.
        frame_x, frame_y = centre(p, FRAME)
        field_x, field_y = centre(p, FIELD)
        frame_w, frame_h = p.js(f"[{FRAME}.clientWidth, {FRAME}.clientHeight]")
        left, top = frame_x - frame_w / 2, frame_y - frame_h / 2

        print("the page's own input")
        p.click(*centre(p, "document.getElementById('own')"))
        p.wait(0.1)
        expect("a press gives it the focus", p.js(OUTER_ACTIVE), "own")
        p.wait(1)
        expect("and it keeps it", p.js(OUTER_ACTIVE), "own")
        p.call("Input.insertText", text="page")
        p.wait(0.5)
        expect("it takes what is typed", p.js("document.getElementById('own').value"), "page")
        expect("the field does not", p.js(f"{FIELD}.value"), "first")

        print("back in the field")
        p.click(left + field_x, top + field_y)
        p.wait(0.8)
        expect("a press gives the frame the focus", p.js(OUTER_ACTIVE), "demo")
        expect("and its input", p.js(INNER_ACTIVE), "INPUT")
        p.call("Input.insertText", text="!")
        p.wait(0.5)
        value = p.js(f"{FIELD}.value")
        expect("the field takes what is typed", "!" in value and "first" in value, True)
        expect("the page's input does not", p.js("document.getElementById('own').value"), "page")
    finally:
        browser.kill()
    return 1 if failures else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1] if len(sys.argv) > 1 else "http://127.0.0.1:8080/"))
