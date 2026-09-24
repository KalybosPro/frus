#!/usr/bin/env python3
"""Checks, in a real browser, that the page's own shortcuts still work while the application
has the keyboard (milestone 569): reload, the developer tools and the address bar are the
browser's, and Tab, Enter, typing and the clipboard keys are the application's.

While the canvas has the focus, winit cancels the default of every key. A key event is
dispatched at the canvas and its return value read: `dispatchEvent` answers `false` when
someone cancelled it. Reload and the rest must answer `true` — nobody cancelled them, so the
browser acts — and the application's own keys `false`.

    pip install websocket-client
    # build the demo for the web and serve it (crates/frus-demo/web/README.md), then:
    python3 scripts/web-keys-check.py http://127.0.0.1:8080/index.html

Needs a browser with WebGPU. Set BROWSER to its executable if it is not found.
"""
import importlib.util, json, os, shutil, subprocess, sys, tempfile, time, urllib.request

HERE = os.path.dirname(os.path.abspath(__file__))
spec = importlib.util.spec_from_file_location("a11y", os.path.join(HERE, "web-accessibility-check.py"))
a11y = importlib.util.module_from_spec(spec)
spec.loader.exec_module(a11y)

# (key, ctrl, shift, is the browser's)
CASES = [
    ("F5", False, False, True), ("F5", True, False, True), ("r", True, False, True),
    ("F12", False, False, True), ("l", True, False, True), ("i", True, True, True),
    ("c", True, False, False), ("v", True, False, False), ("a", True, False, False),
    ("Tab", False, False, False), ("Enter", False, False, False), ("a", False, False, False),
    ("F10", False, True, False),
]
failures = []


def main(base):
    profile = tempfile.mkdtemp(prefix="frus-keys-check-")
    browser = subprocess.Popen([
        a11y.find_browser(), "--headless=new", f"--remote-debugging-port={a11y.PORT}", f"--user-data-dir={profile}",
        "--no-first-run", "--enable-unsafe-webgpu", "--enable-features=Vulkan,WebGPU", "--ignore-gpu-blocklist",
        "--force-renderer-accessibility", "--window-size=1000,800", "about:blank",
    ], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    try:
        for _ in range(50):
            try:
                urllib.request.urlopen(f"http://127.0.0.1:{a11y.PORT}/json", timeout=2)
                break
            except Exception:
                time.sleep(0.2)
        p = a11y.open_at(base)
        p.wait(7)
        for key, ctrl, shift, browsers in CASES:
            js = ("(()=>{const c=document.querySelector('canvas');"
                  "return c.dispatchEvent(new KeyboardEvent('keydown',{key:%s,ctrlKey:%s,shiftKey:%s,bubbles:true,cancelable:true}));})()"
                  % (json.dumps(key), str(ctrl).lower(), str(shift).lower()))
            kept_by_browser = p.js(js)
            name = ("Ctrl+" if ctrl else "") + ("Shift+" if shift else "") + key
            ok = kept_by_browser == browsers
            print(f"  {'ok  ' if ok else 'FAIL'} {name}: {'the browser keeps it' if kept_by_browser else 'the application has it'}"
                  + ("" if ok else f"   (wanted {'the browser' if browsers else 'the application'})"))
            if not ok:
                failures.append(name)
    finally:
        browser.kill()
        shutil.rmtree(profile, ignore_errors=True)
    print("FAILED: " + ", ".join(failures) if failures else "all good")
    return 1 if failures else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1] if len(sys.argv) > 1 else "http://127.0.0.1:8080/index.html"))
