#!/usr/bin/env python3
"""Checks, in a real browser, that the arrow keys move the caret inside a text field that holds
text (milestone 570).

They moved the *focus* to the next control instead: a field with text in it shows a clear button,
and the shell decided whether the focused widget was a text field with a caret probe at a corner
the button covers. In an empty field, which has no button, the arrows worked — which is how it
went unseen. The check types into the demo's add field, moves with Left, Right, Home and End,
types a marker each time and reads the field's value from the accessibility tree.

    pip install websocket-client
    # build the demo for the web and serve it (crates/frus-demo/web/README.md), then:
    python3 scripts/web-caret-keys-check.py http://127.0.0.1:8080/index.html

Needs a browser with WebGPU. Set BROWSER to its executable if it is not found.
"""
import importlib.util, os, shutil, subprocess, sys, tempfile, time, urllib.request

HERE = os.path.dirname(os.path.abspath(__file__))
spec = importlib.util.spec_from_file_location("a11y", os.path.join(HERE, "web-accessibility-check.py"))
a11y = importlib.util.module_from_spec(spec)
spec.loader.exec_module(a11y)

failures = []


def expect(label, got, want):
    ok = got == want
    print(f"  {'ok  ' if ok else 'FAIL'} {label}: {got}" + ("" if ok else f"   (wanted {want})"))
    if not ok:
        failures.append(label)


def typed(p, text):
    for ch in text:
        p.call("Input.dispatchKeyEvent", type="keyDown", key=ch, text=ch)
        p.call("Input.dispatchKeyEvent", type="keyUp", key=ch)


def press(p, name, vk, times=1):
    for _ in range(times):
        p.call("Input.dispatchKeyEvent", type="rawKeyDown", key=name, code=name, windowsVirtualKeyCode=vk)
        p.call("Input.dispatchKeyEvent", type="keyUp", key=name, code=name, windowsVirtualKeyCode=vk)
        p.wait(0.1)


def value(p):
    field = p.find(p.ax_tree(), role="textbox")
    return None if not field else field.get("value", {}).get("value")


def main(base):
    profile = tempfile.mkdtemp(prefix="frus-caret-check-")
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
        field = p.find(p.ax_tree(), role="textbox")
        if not field:
            print("  FAIL no text field found")
            return 1
        p.focus_backend_node(field["backendDOMNodeId"])
        p.wait(0.5)
        typed(p, "hello world")
        p.wait(0.8)
        expect("the text is typed", value(p), "hello world")
        press(p, "ArrowLeft", 37, 3)
        typed(p, "X")
        p.wait(0.6)
        expect("Left three times, then a letter, lands three from the end", value(p), "hello woXrld")
        press(p, "ArrowRight", 39, 2)
        typed(p, "Y")
        p.wait(0.6)
        expect("Right twice, then a letter", value(p), "hello woXrlYd")
        press(p, "Home", 36)
        typed(p, "<")
        press(p, "End", 35)
        typed(p, ">")
        p.wait(0.6)
        expect("Home and End reach the two ends", value(p), "<hello woXrlYd>")
    finally:
        browser.kill()
        shutil.rmtree(profile, ignore_errors=True)
    print("FAILED: " + ", ".join(failures) if failures else "all good")
    return 1 if failures else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1] if len(sys.argv) > 1 else "http://127.0.0.1:8080/index.html"))
