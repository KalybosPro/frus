#!/usr/bin/env python3
"""Checks, in a real browser, that a text field is edited through a real element (milestone 711).

The canvas is pixels, and a field took its text from winit's key events alone: no software
keyboard, no composition, no dictation, nothing for an assistive technology to set. While a
field has the focus the shell now keeps an `<input>` over it, and puts what the browser does
to it back in the field. The check focuses the demo's add field and:

- finds the element: an `<input>`, named like the field, with the browser's focus;
- types, then moves the caret with keys the shell keeps (the arrows, Home and End) and types
  again — where the letters land says the caret went back to the element;
- composes a character as an input method does, and commits it;
- selects everything with the shell's Ctrl+A and types over it — the selection went back too;
- sets the value as an assistive technology does, then submits with Enter: a task appears;
- leaves with Tab: the element goes, and the canvas has the keys again.

    pip install websocket-client
    # build the demo for the web and serve it (crates/frus-demo/web/README.md), then:
    python3 scripts/web-text-editing-check.py http://127.0.0.1:8080/index.html

Needs a browser with WebGPU. Set BROWSER to its executable if it is not found.
"""
import importlib.util, os, shutil, subprocess, sys, tempfile, time, urllib.request

HERE = os.path.dirname(os.path.abspath(__file__))
spec = importlib.util.spec_from_file_location("a11y", os.path.join(HERE, "web-accessibility-check.py"))
a11y = importlib.util.module_from_spec(spec)
spec.loader.exec_module(a11y)

# The check prints the characters it composes, whatever the console's code page.
sys.stdout.reconfigure(encoding="utf-8")

failures = []


def expect(label, got, want):
    ok = got == want
    print(f"  {'ok  ' if ok else 'FAIL'} {label}: {got!r}" + ("" if ok else f"   (wanted {want!r})"))
    if not ok:
        failures.append(label)


def typed(p, text):
    for ch in text:
        p.call("Input.dispatchKeyEvent", type="keyDown", key=ch, text=ch)
        p.call("Input.dispatchKeyEvent", type="keyUp", key=ch)
    p.wait(0.5)


def press(p, name, vk, times=1, modifiers=0):
    for _ in range(times):
        p.call("Input.dispatchKeyEvent", type="rawKeyDown", key=name, code=name,
               windowsVirtualKeyCode=vk, modifiers=modifiers)
        p.call("Input.dispatchKeyEvent", type="keyUp", key=name, code=name,
               windowsVirtualKeyCode=vk, modifiers=modifiers)
        p.wait(0.15)
    p.wait(0.4)


def with_ctrl(p, key, vk):
    """Ctrl and a key, as a keyboard sends them: Control goes down first."""
    p.call("Input.dispatchKeyEvent", type="rawKeyDown", key="Control", code="ControlLeft",
           windowsVirtualKeyCode=17, modifiers=2)
    p.call("Input.dispatchKeyEvent", type="rawKeyDown", key=key, code="Key" + key.upper(),
           windowsVirtualKeyCode=vk, modifiers=2)
    p.call("Input.dispatchKeyEvent", type="keyUp", key=key, code="Key" + key.upper(),
           windowsVirtualKeyCode=vk, modifiers=2)
    p.call("Input.dispatchKeyEvent", type="keyUp", key="Control", code="ControlLeft",
           windowsVirtualKeyCode=17)
    p.wait(0.5)


def field_value(p):
    """What the field holds, as the accessibility tree reads it from the canvas's own node."""
    field = p.find(p.ax_tree(), role="textbox", name_contains="New task")
    # An empty field has no value in the tree at all.
    return None if not field else field.get("value", {}).get("value", "")


ACTIVE = "(() => { const a = document.activeElement; return a && a.tagName; })()"
ELEMENT = "document.querySelector('[data-frus-field]')"


def main(base):
    profile = tempfile.mkdtemp(prefix="frus-editing-check-")
    browser = subprocess.Popen([
        a11y.find_browser(), "--headless=new", f"--remote-debugging-port={a11y.PORT}", f"--user-data-dir={profile}",
        "--no-first-run", "--enable-unsafe-webgpu", "--enable-features=Vulkan,WebGPU", "--ignore-gpu-blocklist",
        "--force-renderer-accessibility", "--window-size=1000,800",
        # A headless page is sometimes put in the background — hidden, its frames stopped — and
        # a key the shell handles then shows nothing until the next frame, which never comes.
        "--disable-renderer-backgrounding", "--disable-background-timer-throttling",
        "--disable-backgrounding-occluded-windows", "about:blank",
    ], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    try:
        for _ in range(50):
            try:
                urllib.request.urlopen(f"http://127.0.0.1:{a11y.PORT}/json", timeout=2)
                break
            except Exception:
                time.sleep(0.2)
        p = a11y.open_at(base)
        p.call("Emulation.setFocusEmulationEnabled", enabled=True)
        p.wait(7)
        field = p.find(p.ax_tree(), role="textbox", name_contains="New task")
        if not field:
            print("  FAIL no field named 'New task' found")
            return 1
        expect("no editing element before the field has the focus", p.js(f"!!{ELEMENT}"), False)
        p.focus_backend_node(field["backendDOMNodeId"])
        p.wait(0.8)

        print("the element")
        expect("the browser's focus is on an input", p.js(ACTIVE), "INPUT")
        expect("it is the field's", p.js(f"document.activeElement === {ELEMENT}"), True)
        expect("named like the field", p.js(f"{ELEMENT}.getAttribute('aria-label')"), "New task")
        expect("prose: sentences capitalised", p.js(f"{ELEMENT}.getAttribute('autocapitalize')"), "sentences")
        expect("outside the canvas", p.js(f"{ELEMENT}.closest('canvas') === null"), True)
        expect("laid out", p.js(f"{ELEMENT}.getBoundingClientRect().width > 0"), True)

        print("typing, and the keys the shell keeps")
        typed(p, "hello world")
        expect("the text is typed", field_value(p), "hello world")
        expect("into the element too", p.js(f"{ELEMENT}.value"), "hello world")
        press(p, "ArrowLeft", 37, 3)
        typed(p, "X")
        expect("Left three times moves the element's caret too", field_value(p), "hello woXrld")
        press(p, "Home", 36)
        typed(p, "<")
        press(p, "End", 35)
        typed(p, ">")
        expect("Home and End", field_value(p), "<hello woXrld>")

        print("composition")
        p.call("Input.imeSetComposition", text="ni", selectionStart=2, selectionEnd=2)
        p.wait(0.5)
        expect("the composition is in the field while it is open", field_value(p), "<hello woXrld>ni")
        p.call("Input.insertText", text="你")
        p.wait(0.6)
        expect("and the committed character replaces it", field_value(p), "<hello woXrld>你")

        print("fast typing")
        # Letters the browser types and a key the shell takes, with no pause between: the
        # arrow must move a caret that already has the letters.
        for ch in "12":
            p.call("Input.dispatchKeyEvent", type="keyDown", key=ch, text=ch)
            p.call("Input.dispatchKeyEvent", type="keyUp", key=ch)
        p.call("Input.dispatchKeyEvent", type="rawKeyDown", key="ArrowLeft", code="ArrowLeft", windowsVirtualKeyCode=37)
        p.call("Input.dispatchKeyEvent", type="keyUp", key="ArrowLeft", code="ArrowLeft", windowsVirtualKeyCode=37)
        p.call("Input.dispatchKeyEvent", type="keyDown", key="3", text="3")
        p.call("Input.dispatchKeyEvent", type="keyUp", key="3")
        p.wait(0.8)
        expect("an arrow straight after two letters lands between them", field_value(p), "<hello woXrld>你132")

        print("selection")
        with_ctrl(p, "a", 65)  # the shell's select-all
        typed(p, "Z")
        expect("Ctrl+A then a letter types over everything", field_value(p), "Z")
        press(p, "Backspace", 8)
        expect("Backspace deletes", field_value(p), "")

        print("an assistive technology sets the value, and Enter submits")
        p.js(f"(() => {{ const e = {ELEMENT}; e.value = 'Buy bread'; e.setSelectionRange(9, 9);"
             " e.dispatchEvent(new InputEvent('input', {inputType: 'insertReplacementText', bubbles: true})); })()")
        p.wait(0.6)
        expect("the value is the field's", field_value(p), "Buy bread")
        press(p, "Enter", 13)
        p.wait(0.6)
        tree = p.ax_tree()
        expect("a task named after it appears", bool(p.find(tree, role="checkbox", name_contains="Buy bread")), True)
        expect("and the field is empty again", field_value(p), "")

        print("leaving")
        press(p, "Tab", 9)
        p.wait(0.6)
        expect("Tab takes the focus out of the field", p.js(f"{ELEMENT} && {ELEMENT}.getAttribute('aria-label')"), None)
        expect("and the page has an element with the focus", p.js(ACTIVE) not in (None, "BODY"), True)
        expect("(the page stayed in the foreground)", p.js("document.visibilityState"), "visible")
    finally:
        browser.kill()
        subprocess.run(["taskkill", "/F", "/T", "/PID", str(browser.pid)], capture_output=True)
        shutil.rmtree(profile, ignore_errors=True)
    print("FAILED: " + ", ".join(failures) if failures else "all good")
    return 1 if failures else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1] if len(sys.argv) > 1 else "http://127.0.0.1:8080/index.html"))
