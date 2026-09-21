#!/usr/bin/env python3
"""Checks the demo's addresses in a real browser (milestones 560-562).

What the unit tests cannot say: that `pushState`, `replaceState`, `history.go` and the
`popstate` listener do what the state machine in `crates/frus-shell/src/history.rs` assumes,
and that the page on screen follows the address. It drives a headless Chromium (Edge or
Chrome) through the DevTools protocol and reads `location` and `history` after each move.

    pip install websocket-client
    # build the demo for the web and serve it, as crates/frus-demo/web/README.md says, then:
    python3 scripts/web-address-check.py http://127.0.0.1:8080/index.html

Needs a browser with WebGPU. Set BROWSER to its executable if it is not found.
"""
import json, os, shutil, subprocess, sys, tempfile, time, urllib.request

import websocket

PORT = 9333
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

    def call(self, method, **params):
        self.n += 1
        self.ws.send(json.dumps({"id": self.n, "method": method, "params": params}))
        while True:
            msg = json.loads(self.ws.recv())
            if msg.get("id") == self.n:
                if "error" in msg:
                    raise RuntimeError(msg["error"])
                return msg.get("result", {})

    def js(self, expression):
        return self.call("Runtime.evaluate", expression=expression, returnByValue=True)["result"].get("value")

    def picture(self):
        """A fingerprint of what is on screen."""
        import hashlib
        return hashlib.sha1(self.call("Page.captureScreenshot", format="png")["data"].encode()).hexdigest()

    def wait(self, seconds):
        time.sleep(seconds)

    def where(self):
        r = json.loads(self.js("JSON.stringify({hash: location.hash, length: history.length, state: history.state})"))
        return r["hash"], r["state"], r["length"]

    def click(self, x, y):
        for kind in ("mousePressed", "mouseReleased"):
            self.call("Input.dispatchMouseEvent", type=kind, x=x, y=y, button="left", clickCount=1)

    def type(self, text):
        for ch in text:
            self.call("Input.dispatchKeyEvent", type="keyDown", key=ch, text=ch)
            self.call("Input.dispatchKeyEvent", type="keyUp", key=ch)
        self.call("Input.dispatchKeyEvent", type="keyDown", key="Enter", code="Enter", windowsVirtualKeyCode=13, text="\r")
        self.call("Input.dispatchKeyEvent", type="keyUp", key="Enter", code="Enter", windowsVirtualKeyCode=13)


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


def main(base):
    profile = tempfile.mkdtemp(prefix="frus-web-check-")
    browser = subprocess.Popen([
        find_browser(), "--headless=new", f"--remote-debugging-port={PORT}", f"--user-data-dir={profile}",
        "--no-first-run", "--enable-unsafe-webgpu", "--enable-features=Vulkan,WebGPU", "--ignore-gpu-blocklist",
        "--window-size=420,800", "about:blank",
    ], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    try:
        for _ in range(50):
            try:
                tabs = json.load(urllib.request.urlopen(f"http://127.0.0.1:{PORT}/json", timeout=2))
                break
            except Exception:
                time.sleep(0.2)
        for tab in tabs:  # the page under test is to be the only one
            if tab["type"] == "page":
                urllib.request.urlopen(f"http://127.0.0.1:{PORT}/json/close/{tab['id']}")
        # `history.length` counts the blank page the tab started at, so it is not asserted.

        print("a bare page: the address is filled in from where the application starts")
        p = open_at(base)
        p.wait(7)
        hash_, state, _ = p.where()
        expect("address", hash_, "#/")
        expect("entry number", state, 0)

        print("a page opened deep: the page beneath gets an entry of its own")
        p = open_at(base + "#/settings")
        p.wait(7)
        hash_, state, length = p.where()
        expect("address", (hash_, state), ("#/settings", 1))
        on_settings = p.picture()
        p.js("history.back()"); p.wait(2.5)
        expect("browser back lands on home", p.where()[:2], ("#/", 0))
        expect("and the screen follows", p.picture() != on_settings, True)
        on_home = p.picture()
        p.js("history.forward()"); p.wait(2.5)
        expect("browser forward comes back", p.where()[:2], ("#/settings", 1))
        expect("and the screen follows again", p.picture() != on_home, True)
        expect("and no entry was added by either", p.where()[2], length)

        print("a reload comes back to the same page, and makes no entries")
        p.js("location.reload()"); p.wait(8)
        expect("address", p.where()[:2], ("#/settings", 1))
        expect("entries", p.where()[2], length)

        print("the application's own moves: a push is an entry, its back goes back")
        p = open_at(base)
        p.wait(7)
        p.click(350, 287); p.wait(1)
        p.type("Buy milk"); p.wait(2)
        p.click(238, 477); p.wait(2.5)  # the avatar opens the task's own screen
        hash_, state, length = p.where()
        expect("the task's address", (hash_, state), ("#/task/0", 1))
        p.click(116, 648); p.wait(2.5)  # its Back button
        expect("back goes back in the list", p.where()[:2], ("#/", 0))
        expect("and adds nothing", p.where()[2], length)
        p.js("history.forward()"); p.wait(2.5)
        expect("forward returns to the task", p.where()[:2], ("#/task/0", 1))
    finally:
        browser.kill()
        shutil.rmtree(profile, ignore_errors=True)
    print("FAILED: " + ", ".join(failures) if failures else "all good")
    return 1 if failures else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1] if len(sys.argv) > 1 else "http://127.0.0.1:8080/index.html"))
