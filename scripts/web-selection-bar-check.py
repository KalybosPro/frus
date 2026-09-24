#!/usr/bin/env python3
"""Checks, in a real browser, that the selection bar can be opened and operated with no
pointer (milestone 568, #23): the context-menu key opens it, its buttons are in the
accessibility tree, and activating one through that tree does what a press would — and
the keys that follow still reach the application.

Driven over the same DevTools protocol as `web-accessibility-check.py`, whose helpers it
reuses:

    pip install websocket-client
    # build the demo for the web and serve it (crates/frus-demo/web/README.md), then:
    python3 scripts/web-selection-bar-check.py http://127.0.0.1:8080/index.html

Needs a browser with WebGPU. Set BROWSER to its executable if it is not found.
"""
import importlib.util, os, shutil, subprocess, sys, tempfile, time, urllib.request

HERE = os.path.dirname(os.path.abspath(__file__))
spec = importlib.util.spec_from_file_location("a11y", os.path.join(HERE, "web-accessibility-check.py"))
a11y = importlib.util.module_from_spec(spec)
spec.loader.exec_module(a11y)

failures = []
BAR = ("Cut", "Copy", "Paste", "Select all")


def expect_true(label, ok):
    print(f"  {'ok  ' if ok else 'FAIL'} {label}")
    if not ok:
        failures.append(label)


def buttons(tree):
    """The names of the buttons the browser's accessibility engine computes for the page."""
    return [
        node.get("name", {}).get("value", "")
        for node in tree
        if not node.get("ignored") and node.get("role", {}).get("value") == "button"
    ]


def bar(tree):
    return [name for name in buttons(tree) if name in BAR]


def typed(p, text):
    for ch in text:
        p.call("Input.dispatchKeyEvent", type="keyDown", key=ch, text=ch)
        p.call("Input.dispatchKeyEvent", type="keyUp", key=ch)


def key(p, name, code, vk):
    p.call("Input.dispatchKeyEvent", type="rawKeyDown", key=name, code=code, windowsVirtualKeyCode=vk)
    p.call("Input.dispatchKeyEvent", type="keyUp", key=name, code=code, windowsVirtualKeyCode=vk)


def main(base):
    profile = tempfile.mkdtemp(prefix="frus-bar-check-")
    browser = subprocess.Popen([
        a11y.find_browser(), "--headless=new", f"--remote-debugging-port={a11y.PORT}", f"--user-data-dir={profile}",
        "--no-first-run", "--enable-unsafe-webgpu", "--enable-features=Vulkan,WebGPU", "--ignore-gpu-blocklist",
        "--force-renderer-accessibility", "--window-size=420,800", "about:blank",
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

        print("a field with text in it, its caret at the end")
        field = p.find(p.ax_tree(), role="textbox")
        expect_true("the add field is exposed as a textbox", field is not None)
        if field:
            p.focus_backend_node(field["backendDOMNodeId"])
            p.wait(0.5)
        typed(p, "hello world")
        p.wait(1)
        expect_true("no bar before it is asked for", bar(p.ax_tree()) == [])

        print("the context-menu key opens the bar")
        key(p, "ContextMenu", "ContextMenu", 93)
        p.wait(1.5)
        opened = bar(p.ax_tree())
        expect_true("Paste is offered (the web cannot tell the clipboard is empty)", "Paste" in opened)
        expect_true("Select all is offered (nothing is selected yet)", "Select all" in opened)
        expect_true("Cut and Copy are not (no selection)", "Cut" not in opened and "Copy" not in opened)

        print("activating Select all through the accessibility tree")
        node = p.find(p.ax_tree(), role="button", name_contains="Select all")
        if node:
            p.focus_backend_node(node["backendDOMNodeId"])
            p.wait(0.3)
            p.activate()
            p.wait(1.5)
        after = bar(p.ax_tree())
        expect_true("the bar stays open, now on everything selected", "Copy" in after)
        expect_true("...offering Cut, Copy and Paste", all(name in after for name in ("Cut", "Copy", "Paste")))
        expect_true("...and no longer Select all", "Select all" not in after)
        typed_after = p.find(p.ax_tree(), role="textbox")
        expect_true("the field was not submitted by the Enter that pressed the button",
                    typed_after is not None and "hello world" in json_text(typed_after))

        print("Escape puts the bar away")
        key(p, "Escape", "Escape", 27)
        p.wait(1.2)
        expect_true("no bar after Escape (the keys still reach the application)", bar(p.ax_tree()) == [])
    finally:
        browser.kill()
        shutil.rmtree(profile, ignore_errors=True)
    print("FAILED: " + ", ".join(failures) if failures else "all good")
    return 1 if failures else 0


def json_text(node):
    """Everything a node says about itself, for a loose 'is that text in there'."""
    return str(node.get("name", {}).get("value", "")) + str(node.get("value", {}).get("value", ""))


if __name__ == "__main__":
    sys.exit(main(sys.argv[1] if len(sys.argv) > 1 else "http://127.0.0.1:8080/index.html"))
