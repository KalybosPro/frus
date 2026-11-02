#!/usr/bin/env python3
"""Checks, in a real browser, that the canvas is the size of a screen smaller than a window's
minimum (milestone 735).

The shell asks a new window for a minimum of 360 by 280, so that nobody drags a desktop window to
something absurd. winit carries that to the canvas on the web as `min-width` and `min-height`,
which held it wider than a small phone's screen, and everything past that edge was cut off. The
check opens the demo at 320 by 240, then in a frame of 300 by 200 on a larger page, and expects the
canvas to be as wide as that and to end inside it. (Not as tall: the demo's page has a bar of its
own above the canvas.)

    pip install websocket-client
    # build the demo for the web and serve it (crates/frus-demo/web/README.md), then:
    python3 scripts/web-small-screen-check.py http://127.0.0.1:8080/

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


def width(p, document):
    return p.js(f"{document}.querySelector('canvas').clientWidth")


def ends_inside(p, document):
    return p.js(f"(() => {{ const d = {document}; const r = d.querySelector('canvas').getBoundingClientRect(); "
                "return r.bottom <= d.defaultView.innerHeight; })()")


def main(base):
    profile = tempfile.mkdtemp(prefix="frus-small-screen-check-")
    browser = subprocess.Popen([
        a11y.find_browser(), "--headless=new", f"--remote-debugging-port={a11y.PORT}", f"--user-data-dir={profile}",
        "--no-first-run", "--enable-unsafe-webgpu", "--enable-features=Vulkan,WebGPU", "--ignore-gpu-blocklist",
        "--window-size=1000,800", *a11y.FOREGROUND, "about:blank",
    ], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    try:
        for _ in range(50):
            try:
                urllib.request.urlopen(f"http://127.0.0.1:{a11y.PORT}/json", timeout=2)
                break
            except Exception:
                time.sleep(0.2)
        root = base.rstrip("/")
        p = a11y.open_at(root + "/no-such-page-small-screen-check")

        print("a screen of 320 by 240")
        p.call("Emulation.setDeviceMetricsOverride", width=320, height=240, deviceScaleFactor=1, mobile=True)
        p.call("Page.navigate", url=root + "/index.html")
        p.wait(7)
        expect("the canvas is as wide as the screen", width(p, "document"), 320)
        expect("and ends inside it", ends_inside(p, "document"), True)
        expect("the page does not scroll sideways", p.js("document.documentElement.scrollWidth"), 320)

        print("a frame of 300 by 200")
        p.call("Emulation.setDeviceMetricsOverride", width=1000, height=800, deviceScaleFactor=1, mobile=False)
        p.call("Page.navigate", url=root + "/no-such-page-small-screen-check")
        p.wait(1)
        p.js("document.body.innerHTML = '<iframe id=\"demo\" style=\"width:300px;height:200px;border:0\"></iframe>'; "
             f"document.getElementById('demo').src = {root + '/index.html'!r}; 0")
        p.wait(7)
        inner = "document.getElementById('demo').contentDocument"
        expect("the canvas is as wide as the frame", width(p, inner), 300)
        expect("and ends inside it", ends_inside(p, inner), True)
    finally:
        browser.kill()
    return 1 if failures else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1] if len(sys.argv) > 1 else "http://127.0.0.1:8080/"))
