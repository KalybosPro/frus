#!/usr/bin/env python3
"""Checks, in a real browser, that `Permission` asks and reads what the browser says (milestone 730).

The widget gallery's Permission demo has, for each permission, a "Look" button (its status, asking
nothing) and an "Ask" button (a request). The check drives them through the accessibility tree in
headless Edge, with a fake camera and microphone that the browser accepts without a question, and
the DevTools protocol granting or refusing what the browser would otherwise ask the person:

- looking at the camera before anything is asked says it is not decided;
- asking for the camera and the microphone opens them for a moment and answers granted;
- the location, granted by the browser and given a position, answers granted;
- notifications, refused by the browser, read as blocked — a page cannot ask again.

    pip install websocket-client
    # build frus-examples/widgets for the web (its README, "Web") and serve it, then:
    python3 scripts/web-permissions-check.py http://127.0.0.1:8080/index.html

Set BROWSER to the browser's executable if it is not found.
"""
import importlib.util, os, subprocess, sys, tempfile, time, urllib.request
from urllib.parse import urlsplit

HERE = os.path.dirname(os.path.abspath(__file__))
spec = importlib.util.spec_from_file_location("a11y", os.path.join(HERE, "web-accessibility-check.py"))
a11y = importlib.util.module_from_spec(spec)
spec.loader.exec_module(a11y)

failures = []
ORDER = ["Camera", "Microphone", "Location", "Notifications"]


def expect(label, got, want):
    ok = got == want
    print(f"  {'ok  ' if ok else 'FAIL'} {label}: {got!r}" + ("" if ok else f"   (wanted {want!r})"))
    if not ok:
        failures.append(label)


def names(p):
    return [n.get("name", {}).get("value", "") for n in p.ax_tree() if not n.get("ignored")]


def said(p, permission, seconds=5.0, before="not looked at yet"):
    """What the demo says of `permission`, waiting for it to say something other than `before`."""
    end = time.time() + seconds
    last = None
    while True:
        for name in names(p):
            if name.startswith(permission + ": "):
                last = name.split(": ", 1)[1]
        if last is not None and last != before:
            return last
        if time.time() >= end:
            return last
        time.sleep(0.25)


def press(p, label, index):
    """Presses the `index`th button named `label`, in the order the page lists them."""
    buttons = [n for n in p.ax_tree()
               if not n.get("ignored")
               and n.get("role", {}).get("value") == "button"
               and n.get("name", {}).get("value") == label]
    if index >= len(buttons):
        failures.append(f"no button {label} #{index}")
        print(f"  FAIL no button {label!r} number {index}")
        return
    p.focus_backend_node(buttons[index]["backendDOMNodeId"])
    p.wait(0.3)
    p.activate()


def main(page_url):
    origin = "{0.scheme}://{0.netloc}".format(urlsplit(page_url))
    profile = tempfile.mkdtemp(prefix="frus-permissions-check-")
    browser = subprocess.Popen([
        a11y.find_browser(), "--headless=new", f"--remote-debugging-port={a11y.PORT}", f"--user-data-dir={profile}",
        "--no-first-run", "--enable-unsafe-webgpu", "--enable-features=Vulkan,WebGPU", "--ignore-gpu-blocklist",
        "--force-renderer-accessibility", "--window-size=1000,900",
        # A camera and a microphone that exist, and a browser that opens them without asking.
        "--use-fake-device-for-media-stream", "--use-fake-ui-for-media-stream",
        *a11y.FOREGROUND, "about:blank",
    ], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    try:
        for _ in range(50):
            try:
                urllib.request.urlopen(f"http://127.0.0.1:{a11y.PORT}/json", timeout=2)
                break
            except Exception:
                time.sleep(0.2)
        p = a11y.open_at(page_url + "#/widget/permissions")
        if not any(n.startswith("Camera: ") for n in names(p)) and not a11y_wait(p, "Camera: ", 15):
            print("  FAIL the demo never showed")
            return 1

        print("looking, before anything is asked")
        press(p, "Look", ORDER.index("Location"))
        expect("the location is not decided", said(p, "Location"), "not decided: asking shows the question")

        print("asking for the camera and the microphone")
        press(p, "Ask", ORDER.index("Camera"))
        expect("the camera, opened for a moment, is granted", said(p, "Camera", 10), "granted")
        press(p, "Ask", ORDER.index("Microphone"))
        expect("the microphone too", said(p, "Microphone", 10), "granted")

        print("asking for the location, which the browser grants")
        p.call("Browser.grantPermissions", origin=origin, permissions=["geolocation"])
        p.call("Emulation.setGeolocationOverride", latitude=5.36, longitude=-4.01, accuracy=10)
        press(p, "Ask", ORDER.index("Location"))
        expect("the location is granted",
               said(p, "Location", 10, before="not decided: asking shows the question"), "granted")

        print("notifications, which the browser refuses")
        p.call("Browser.setPermission", origin=origin, permission={"name": "notifications"}, setting="denied")
        press(p, "Look", ORDER.index("Notifications"))
        expect("refused notifications are blocked", said(p, "Notifications"), "blocked: only the settings can change it")
        return 1 if failures else 0
    finally:
        browser.kill()
        subprocess.run(["taskkill", "/F", "/T", "/PID", str(browser.pid)], capture_output=True)


def a11y_wait(p, prefix, seconds):
    end = time.time() + seconds
    while time.time() < end:
        if any(n.startswith(prefix) for n in names(p)):
            return True
        time.sleep(0.25)
    return False


if __name__ == "__main__":
    code = main(sys.argv[1] if len(sys.argv) > 1 else "http://127.0.0.1:8080/index.html")
    print("FAILED: " + ", ".join(failures) if failures else "all checks passed")
    sys.exit(code)
