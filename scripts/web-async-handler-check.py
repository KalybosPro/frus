#!/usr/bin/env python3
"""Checks, in a real browser, that an `async` handler runs around what it awaits (milestone 714).

The http example's "New post" form sends with `cx.async_callback`: the handler locks the form,
awaits the POST, and shows the answer. On the web the task runs on the browser's own loop and
the request is the browser's `fetch`. The check holds the POST in the browser (the DevTools
protocol's Fetch domain) so that it can look at the page while the handler is waiting:

- the form is filled and Send pressed, through the accessibility tree;
- while the request is held, the page shows "Sending…": the handler changed the state before
  its `await`, and the frames go on while it waits;
- the request is let go: the page shows what the server created;
- a second send is held, and the page left while it waits: the task ends with the page, and
  the answer that comes after changes nothing and breaks nothing.

    pip install websocket-client
    # build frus-examples/http for the web (its README, "Web") and serve it, then:
    python3 scripts/web-async-handler-check.py http://127.0.0.1:8080/index.html

Needs a browser with WebGPU and the network (the example talks to jsonplaceholder.typicode.com).
Set BROWSER to its executable if it is not found.
"""
import importlib.util, json, os, subprocess, sys, tempfile, time, urllib.request

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


class Page(a11y.Page):
    """A page that keeps the events it is sent, instead of dropping them."""

    def __init__(self, ws_url):
        self.events = []
        super().__init__(ws_url)

    def call(self, method, **params):
        self.n += 1
        self.ws.send(json.dumps({"id": self.n, "method": method, "params": params}))
        while True:
            msg = json.loads(self.ws.recv())
            if msg.get("id") == self.n:
                if "error" in msg:
                    raise RuntimeError(f"{method}: {msg['error']}")
                return msg.get("result", {})
            if "method" in msg:
                self.events.append(msg)

    def paused_post(self, seconds):
        """The POST the Fetch domain holds, once it is held; other requests are let through."""
        end = time.time() + seconds
        while time.time() < end:
            self.js("0")  # reads what the browser has sent meanwhile
            for event in list(self.events):
                if event["method"] != "Fetch.requestPaused":
                    continue
                self.events.remove(event)
                request_id = event["params"]["requestId"]
                if event["params"]["request"]["method"] == "POST":
                    return request_id
                self.call("Fetch.continueRequest", requestId=request_id)
            time.sleep(0.1)
        return None


def names(p):
    return [n.get("name", {}).get("value", "") for n in p.ax_tree() if not n.get("ignored")]


def has(p, text, seconds=0.0):
    end = time.time() + seconds
    while True:
        if any(text in name for name in names(p)):
            return True
        if time.time() >= end:
            return False
        time.sleep(0.25)


def fill(p, label, text):
    field = p.find(p.ax_tree(), role="textbox", name_contains=label)
    if not field:
        failures.append(f"no field {label}")
        print(f"  FAIL no field named {label!r}")
        return
    p.focus_backend_node(field["backendDOMNodeId"])
    p.wait(0.6)
    for ch in text:
        p.call("Input.dispatchKeyEvent", type="keyDown", key=ch, text=ch)
        p.call("Input.dispatchKeyEvent", type="keyUp", key=ch)
    p.wait(0.4)


def press(p, role, name):
    node = p.find(p.ax_tree(), role=role, name_contains=name)
    if not node:
        failures.append(f"no {role} {name}")
        print(f"  FAIL no {role} named {name!r}")
        return
    p.focus_backend_node(node["backendDOMNodeId"])
    p.wait(0.3)
    p.activate()


def send(p):
    fill(p, "Title", "Hello")
    fill(p, "Body", "Sent from an async handler.")
    p.call("Fetch.enable", patterns=[{"urlPattern": "*jsonplaceholder*", "requestStage": "Request"}])
    press(p, "button", "Send")
    return p.paused_post(10)


def main(base):
    profile = tempfile.mkdtemp(prefix="frus-async-check-")
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
        a11y.Page = Page  # open_at makes its page with this
        p = a11y.open_at(base + "#/posts/new")
        p.call("Runtime.evaluate", expression="window.errors = []; addEventListener('error', e => errors.push(String(e.message)))")
        if not has(p, "Title", 15):
            print("  FAIL the form never showed")
            return 1

        print("a send, held while the handler waits")
        held = send(p)
        expect("the POST went out", held is not None, True)
        expect("while it is held, the page says it is sending", has(p, "Sending", 3), True)
        p.wait(1.0)
        expect("and still does a second later: the task waits, the frames go on", has(p, "Sending"), True)
        if held:
            p.call("Fetch.continueRequest", requestId=held)
        expect("let go, the answer is shown", has(p, "Created", 10), True)
        expect("and the form is no longer locked", has(p, "Sending"), False)

        print("a send the page does not wait for")
        p.call("Page.close")
        # A tab of its own, on the list; the form is reached through the address, so going
        # back is going back to the list.
        p = a11y.open_at(base + "#/")
        p.wait(4)
        p.js("location.hash = '#/posts/new'")
        if not has(p, "Title", 15):
            print("  FAIL the form never showed again")
            return 1
        p.call("Runtime.evaluate", expression="window.errors = []; addEventListener('error', e => errors.push(String(e.message)))")
        held = send(p)
        expect("the POST went out", held is not None, True)
        expect("the page says it is sending", has(p, "Sending", 3), True)
        p.js("history.back()")
        # The page slides away: give the transition its time.
        end = time.time() + 3
        while has(p, "Sending") and time.time() < end:
            time.sleep(0.25)
        expect("the page is left while it waits", has(p, "Sending"), False)
        if held:
            p.call("Fetch.continueRequest", requestId=held)
        p.wait(2.0)
        expect("the answer that comes after shows nothing", has(p, "Created"), False)
        expect("and breaks nothing", p.js("errors"), [])
        return 1 if failures else 0
    finally:
        browser.kill()
        subprocess.run(["taskkill", "/F", "/T", "/PID", str(browser.pid)], capture_output=True)


if __name__ == "__main__":
    code = main(sys.argv[1] if len(sys.argv) > 1 else "http://127.0.0.1:8080/index.html")
    print("FAILED: " + ", ".join(failures) if failures else "all checks passed")
    sys.exit(code)
