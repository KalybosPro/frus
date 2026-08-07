package dev.frus.input;

import android.app.Activity;
import android.content.Context;
import android.text.InputType;
import android.view.KeyEvent;
import android.view.View;
import android.view.ViewGroup;
import android.view.inputmethod.BaseInputConnection;
import android.view.inputmethod.EditorInfo;
import android.view.inputmethod.ExtractedText;
import android.view.inputmethod.ExtractedTextRequest;
import android.view.inputmethod.InputConnection;
import android.view.inputmethod.InputMethodManager;

/**
 * The frus input bridge (see docs/jalon-81.md): NativeActivity offers no
 * InputConnection at all, so IMEs run in a degraded mode (TYPE_NULL, Latin keys
 * only, with no composition, no swipe and no CJK). This focusable 1x1 View,
 * added on top of the native content, supplies a real InputConnection and
 * relays every IME operation to the native code, through the `native*` methods
 * registered by RegisterNatives on the Rust side.
 *
 * Compiled into a bundled dex (scripts/build-input-dex.sh) and loaded at
 * runtime by InMemoryDexClassLoader, so the packaging never changes.
 */
public final class FrusTextBridge extends View {
    private static native void nativeCommit(String text);
    private static native void nativeSetComposing(String text);
    private static native void nativeFinishComposing();
    private static native void nativeDelete(int before, int after);
    private static native void nativeEditorAction(int action);
    /** Returns true when the native side consumed the key, that is, edited. */
    private static native boolean nativeKey(int keyCode, boolean down, int unicode, int meta);
    // The input context: the focused field's real editing state, so that the IME
    // offers suggestions that make sense.
    private static native String nativeTextBeforeCursor(int n);
    private static native String nativeTextAfterCursor(int n);
    private static native String nativeSelectedText();

    private static FrusTextBridge instance;

    /** Adds the bridge view to the activity, once and once only. */
    public static void install(final Activity activity) {
        activity.runOnUiThread(new Runnable() {
            @Override
            public void run() {
                if (instance == null) {
                    instance = new FrusTextBridge(activity);
                    activity.addContentView(instance, new ViewGroup.LayoutParams(1, 1));
                }
            }
        });
    }

    /** A native text field takes focus: the bridge view captures the IME. */
    public static void startInput(final Activity activity) {
        activity.runOnUiThread(new Runnable() {
            @Override
            public void run() {
                if (instance == null) {
                    return;
                }
                instance.requestFocus();
                InputMethodManager imm =
                        (InputMethodManager) activity.getSystemService(Context.INPUT_METHOD_SERVICE);
                imm.restartInput(instance);
                imm.showSoftInput(instance, 0);
            }
        });
    }

    /** Focus leaves the text fields: the IME closes and focus is released. */
    public static void stopInput(final Activity activity) {
        activity.runOnUiThread(new Runnable() {
            @Override
            public void run() {
                if (instance == null) {
                    return;
                }
                InputMethodManager imm =
                        (InputMethodManager) activity.getSystemService(Context.INPUT_METHOD_SERVICE);
                imm.hideSoftInputFromWindow(instance.getWindowToken(), 0);
                instance.clearFocus();
            }
        });
    }

    private FrusTextBridge(Context context) {
        super(context);
        setFocusable(true);
        setFocusableInTouchMode(true);
    }

    @Override
    public boolean onCheckIsTextEditor() {
        return true;
    }

    @Override
    public InputConnection onCreateInputConnection(EditorInfo out) {
        // Free text WITH composition and suggestions turned on, the context being
        // supplied by the get*Cursor natives: this is what lights up the
        // composition underline and the IME's proposals.
        out.inputType = InputType.TYPE_CLASS_TEXT | InputType.TYPE_TEXT_FLAG_CAP_SENTENCES;
        out.imeOptions = EditorInfo.IME_ACTION_DONE | EditorInfo.IME_FLAG_NO_FULLSCREEN;
        return new Connection(this);
    }

    // Hardware and d-pad keys the focused view receives: editing is consumed on
    // the native side, and the rest — Back and so on — follows the default path
    // through to NativeActivity, for the back gesture and navigation.
    @Override
    public boolean onKeyDown(int keyCode, KeyEvent event) {
        return nativeKey(keyCode, true, event.getUnicodeChar(), event.getMetaState())
                || super.onKeyDown(keyCode, event);
    }

    @Override
    public boolean onKeyUp(int keyCode, KeyEvent event) {
        return nativeKey(keyCode, false, event.getUnicodeChar(), event.getMetaState())
                || super.onKeyUp(keyCode, event);
    }

    /** The real InputConnection: every IME operation goes to the native side. */
    private static final class Connection extends BaseInputConnection {
        Connection(View target) {
            super(target, true);
        }

        @Override
        public boolean commitText(CharSequence text, int newCursorPosition) {
            nativeCommit(text.toString());
            return true;
        }

        @Override
        public boolean setComposingText(CharSequence text, int newCursorPosition) {
            nativeSetComposing(text.toString());
            return true;
        }

        @Override
        public boolean finishComposingText() {
            nativeFinishComposing();
            return true;
        }

        @Override
        public boolean deleteSurroundingText(int beforeLength, int afterLength) {
            nativeDelete(beforeLength, afterLength);
            return true;
        }

        @Override
        public boolean performEditorAction(int actionCode) {
            nativeEditorAction(actionCode);
            return true;
        }

        // Context: the answers come from the native editing state, not from the
        // local Editable, which is empty — hence suggestions and corrections that
        // make sense.
        @Override
        public CharSequence getTextBeforeCursor(int n, int flags) {
            String s = nativeTextBeforeCursor(n);
            return s != null ? s : "";
        }

        @Override
        public CharSequence getTextAfterCursor(int n, int flags) {
            String s = nativeTextAfterCursor(n);
            return s != null ? s : "";
        }

        @Override
        public CharSequence getSelectedText(int flags) {
            String s = nativeSelectedText();
            return (s != null && s.length() > 0) ? s : null;
        }

        // The **complete** editing state: many IMEs, SwiftKey among them, turn on
        // composition and prediction only if they can extract the whole field.
        @Override
        public ExtractedText getExtractedText(ExtractedTextRequest request, int flags) {
            String before = nativeTextBeforeCursor(100000);
            String after = nativeTextAfterCursor(100000);
            ExtractedText et = new ExtractedText();
            et.text = before + after;
            et.startOffset = 0;
            et.selectionStart = before.length();
            et.selectionEnd = before.length();
            et.partialStartOffset = -1;
            et.partialEndOffset = -1;
            return et;
        }

        @Override
        public boolean sendKeyEvent(KeyEvent event) {
            // Some IMEs send Delete and Enter through here rather than through
            // deleteSurroundingText and performEditorAction.
            return nativeKey(
                    event.getKeyCode(),
                    event.getAction() == KeyEvent.ACTION_DOWN,
                    event.getUnicodeChar(),
                    event.getMetaState());
        }
    }
}
