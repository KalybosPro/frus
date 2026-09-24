#!/usr/bin/env python3
"""Checks the demo's accessibility bridge in a real browser (milestone 564, #18).

What no unit test can say: that the DOM the shell projects (`a11y_web.rs`) is what
Chromium's own accessibility engine actually computes, and that the loop closes both
ways — a widget's semantics reach the accessibility tree, and an action taken *through*
that tree (focus, then the browser's own Enter-activates-a-button behaviour) reaches
the widget back. Driven over the DevTools protocol's Accessibility and DOM domains,
which is what a screen reader itself reads from and writes to; this is not a heavier
DOM query, it is the same computed tree NVDA, JAWS and Narrator consume.

    pip install websocket-client
    # build the demo for the web and serve it (crates/frus-demo/web/README.md), then:
    python3 scripts/web-accessibility-check.py http://127.0.0.1:8080/index.html

Needs a browser with WebGPU. Set BROWSER to its executable if it is not found.
"""
import json, os, shutil, subprocess, sys, tempfile, time, urllib.request

import websocket

PORT = 9334
CANDIDATES = [
    r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe",
    r"C:\Program Files\Microsoft\Edge\Application\msedge.exe",
    r"C:\Program Files\Google\Chrome\Application\chrome.exe",
    "microsoft-edge", "google-chrome", "chromium", "chrome",
]


def find_browser():
    if os.environ.get("BROWSER"):
        return os.environ["BROWSER"]
    for candidate in CANDIDATES:
        if os.path.exists(candidate) or shutil.which(candidate):
            return candidate
    sys.exit("no browser found: set BROWSER")


class Page:
    def __init__(self, ws_url):
        self.ws = websocket.create_connection(ws_url, timeout=30, suppress_origin=True)
        self.n = 0
        self.call("Runtime.enable")
        self.call("Page.enable")
        self.call("DOM.enable")
        self.call("Accessibility.enable")

    def call(self, method, **params):
        self.n += 1
        self.ws.send(json.dumps({"id": self.n, "method": method, "params": params}))
        while True:
            msg = json.loads(self.ws.recv())
            if msg.get("id") == self.n:
                if "error" in msg:
                    raise RuntimeError(f"{method}: {msg['error']}")
                return msg.get("result", {})

    def js(self, expression):
        return self.call("Runtime.evaluate", expression=expression, returnByValue=True)["result"].get("value")

    def wait(self, seconds):
        time.sleep(seconds)

    def click(self, x, y):
        for kind in ("mousePressed", "mouseReleased"):
            self.call("Input.dispatchMouseEvent", type=kind, x=x, y=y, button="left", clickCount=1)

    def type(self, text):
        for ch in text:
            self.call("Input.dispatchKeyEvent", type="keyDown", key=ch, text=ch)
            self.call("Input.dispatchKeyEvent", type="keyUp", key=ch)
        self.call("Input.dispatchKeyEvent", type="keyDown", key="Enter", code="Enter", windowsVirtualKeyCode=13, text="\r")
        self.call("Input.dispatchKeyEvent", type="keyUp", key="Enter", code="Enter", windowsVirtualKeyCode=13)

    def press(self, key, code, vk):
        self.call("Input.dispatchKeyEvent", type="rawKeyDown", key=key, code=code, windowsVirtualKeyCode=vk)
        self.call("Input.dispatchKeyEvent", type="keyUp", key=key, code=code, windowsVirtualKeyCode=vk)

    def activate(self):
        """A real Enter on the focused element: a keydown that **carries its character**, which is
        what makes a browser activate a button. A bare `rawKeyDown` does not, and only ever
        looked like it worked because the shell also read the key itself."""
        self.call("Input.dispatchKeyEvent", type="keyDown", key="Enter", code="Enter",
                  windowsVirtualKeyCode=13, text="\r")
        self.call("Input.dispatchKeyEvent", type="keyUp", key="Enter", code="Enter", windowsVirtualKeyCode=13)

    def ax_tree(self):
        """Every node the browser's accessibility engine currently computes for the page."""
        return self.call("Accessibility.getFullAXTree")["nodes"]

    def find(self, tree, role=None, name_contains=None):
        for node in tree:
            if node.get("ignored"):
                continue
            r = node.get("role", {}).get("value")
            nm = node.get("name", {}).get("value", "")
            if role is not None and r != role:
                continue
            if name_contains is not None and name_contains not in nm:
                continue
            return node
        return None

    def prop(self, node, name):
        for p in node.get("properties", []):
            if p.get("name") == name:
                return p.get("value", {}).get("value")
        return None

    def focus_backend_node(self, backend_node_id):
        self.call("DOM.focus", backendNodeId=backend_node_id)


def open_at(url):
    tab = json.load(urllib.request.urlopen(urllib.request.Request(
        f"http://127.0.0.1:{PORT}/json/new?about:blank", method="PUT")))
    page = Page(tab["webSocketDebuggerUrl"])
    page.call("Page.navigate", url=url)
    page.call("Page.bringToFront")
    return page


failures = []


def expect(label, got, want):
    ok = got == want
    print(f"  {'ok  ' if ok else 'FAIL'} {label}: {got}" + ("" if ok else f"   (wanted {want})"))
    if not ok:
        failures.append(label)


def expect_true(label, got):
    print(f"  {'ok  ' if got else 'FAIL'} {label}")
    if not got:
        failures.append(label)


def main(base):
    profile = tempfile.mkdtemp(prefix="frus-a11y-check-")
    browser = subprocess.Popen([
        find_browser(), "--headless=new", f"--remote-debugging-port={PORT}", f"--user-data-dir={profile}",
        "--no-first-run", "--enable-unsafe-webgpu", "--enable-features=Vulkan,WebGPU", "--ignore-gpu-blocklist",
        "--force-renderer-accessibility", "--window-size=420,800", "about:blank",
    ], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    try:
        for _ in range(50):
            try:
                urllib.request.urlopen(f"http://127.0.0.1:{PORT}/json", timeout=2)
                break
            except Exception:
                time.sleep(0.2)

        print("the page loads with the accessibility tree already in place")
        p = open_at(base)
        p.wait(7)
        tree = p.ax_tree()
        root = p.find(tree, role="application")
        expect_true("the canvas is exposed as role=application", root is not None)
        if root:
            expect_true("...with the app's title as its name",
                        bool(root.get("name", {}).get("value")))

        menu = p.find(tree, role="button", name_contains="Menu")
        expect_true("the menu icon button is exposed, named 'Menu'", menu is not None)

        print("adding a task announces it and names the checkbox after it")
        p.click(350, 287); p.wait(1)
        p.type("Wash the dog"); p.wait(2)
        tree = p.ax_tree()
        box = p.find(tree, role="checkbox", name_contains="Wash the dog")
        expect_true("a checkbox named after the new task exists", box is not None)
        if box:
            expect("...unchecked to start", p.prop(box, "checked"), "false")
        # The status container's own accessible name is empty — `role="status"` is not
        # a naming-content role, so what an AT actually reads is exposed as the
        # container's text content, a StaticText child in the computed tree.
        live = p.find(tree, name_contains="Wash the dog added")
        expect_true("a live region carries the 'added' announcement", live is not None)

        print("focusing and activating the checkbox through the accessibility tree ticks it")
        backend_id = box["backendDOMNodeId"]
        p.focus_backend_node(backend_id)
        p.wait(0.3)
        # The element is a real <button>: the platform itself fires `click` on Enter,
        # the same as a screen reader's own "activate" would deliver. The bridge keeps that
        # key from also reaching the shell as typing (milestone 568), so `click` is the one
        # way in, and the Enter has to be a real one.
        p.activate()
        p.wait(1)
        tree = p.ax_tree()
        box = p.find(tree, role="checkbox", name_contains="Wash the dog")
        expect_true("the checkbox is still there after activation", box is not None)
        if box:
            expect("...and is now checked", p.prop(box, "checked"), "true")

        print("a delete button is reachable and named for what it does")
        delete = p.find(tree, role="button", name_contains="Delete task")
        expect_true("a 'Delete task' button exists for the row", delete is not None)
    finally:
        browser.kill()
        shutil.rmtree(profile, ignore_errors=True)
    print("FAILED: " + ", ".join(failures) if failures else "all good")
    return 1 if failures else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1] if len(sys.argv) > 1 else "http://127.0.0.1:8080/index.html"))
