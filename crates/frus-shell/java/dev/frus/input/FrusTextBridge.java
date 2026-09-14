package dev.frus.input;

import android.app.Activity;
import android.content.Context;
import android.graphics.Rect;
import android.os.Build;
import android.text.InputType;
import android.util.SparseArray;
import android.view.KeyEvent;
import android.view.View;
import android.view.ViewGroup;
import android.view.ViewStructure;
import android.view.autofill.AutofillId;
import android.view.autofill.AutofillManager;
import android.view.autofill.AutofillValue;
import android.view.inputmethod.BaseInputConnection;
import android.view.inputmethod.EditorInfo;
import android.view.inputmethod.ExtractedText;
import android.view.inputmethod.ExtractedTextRequest;
import android.view.inputmethod.InputConnection;
import android.view.inputmethod.InputMethodManager;
import java.util.ArrayList;

/**
 * The frus input bridge (see docs/milestone-81.md): NativeActivity offers no
 * InputConnection at all, so IMEs run in a degraded mode (TYPE_NULL, Latin keys
 * only, with no composition, no swipe and no CJK). This focusable 1x1 View,
 * added on top of the native content, supplies a real InputConnection and
 * relays every IME operation to the native code, through the `native*` methods
 * registered by RegisterNatives on the Rust side.
 *
 * <p>It is also where the platform's autofill service meets the native fields
 * (milestone 512): the form being edited is declared as a virtual structure under
 * this view, one child per field, and the values the service hands back go to the
 * native side the way typing does.
 *
 * Compiled into a bundled dex (scripts/build-input-dex.sh) and loaded at
 * runtime by InMemoryDexClassLoader, so the packaging never changes.
 */
public final class FrusTextBridge extends View {
    private static native void nativeCommit(String text);
    private static native void nativeSetComposing(String text);
    private static native void nativeSetComposingRegion(int start, int end);
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
    /** A value the autofill service chose for the field it knows by {@code virtualId}. */
    private static native void nativeAutofill(int virtualId, String value);

    private static FrusTextBridge instance;

    /**
     * What the focused field wants of the keyboard, set by the native side on every
     * startInput. The two values are computed in Rust and applied here unchanged:
     * this file is a pipe, not a policy, so that a new keyboard type never needs a
     * new dex.
     */
    private static int inputType =
            InputType.TYPE_CLASS_TEXT | InputType.TYPE_TEXT_FLAG_CAP_SENTENCES;

    private static int imeOptions = EditorInfo.IME_ACTION_DONE | EditorInfo.IME_FLAG_NO_FULLSCREEN;

    /**
     * One field of the form shown to the autofill service. Every value in it was worked
     * out on the native side -- which fields, their ids, their platform hint names, their
     * boxes in pixels from this view's corner -- and is carried here unchanged, for the
     * same reason the keyboard's two integers are.
     */
    private static final class Field {
        final int id;
        final String[] hints;
        final int left;
        final int top;
        final int width;
        final int height;
        final String value;
        final boolean sensitive;

        Field(int id, String[] hints, int left, int top, int width, int height, String value,
                boolean sensitive) {
            this.id = id;
            this.hints = hints;
            this.left = left;
            this.top = top;
            this.width = width;
            this.height = height;
            this.value = value;
            this.sensitive = sensitive;
        }
    }

    /**
     * The form being assembled by the native side, field by field, and the one last
     * published. The structure is read on the UI thread and written from the native
     * one, so it is built aside and swapped whole: a service asking half-way through a
     * rebuild sees the previous form, never half of the next.
     */
    private static ArrayList<Field> pendingFields = new ArrayList<Field>();
    private static ArrayList<Field> publishedFields = new ArrayList<Field>();

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

    /**
     * A native text field takes focus: the bridge view captures the IME.
     *
     * <p>{@code type} and {@code options} are Android's own InputType and imeOptions
     * bit fields, already assembled on the native side. restartInput below is what
     * makes them take effect for a field focused after another one: without it the
     * IME keeps the EditorInfo it was given first, and every field after the first
     * would inherit the first field's keyboard.
     */
    public static void startInput(final Activity activity, final int type, final int options) {
        activity.runOnUiThread(new Runnable() {
            @Override
            public void run() {
                if (instance == null) {
                    return;
                }
                inputType = type;
                imeOptions = options;
                instance.requestFocus();
                InputMethodManager imm =
                        (InputMethodManager) activity.getSystemService(Context.INPUT_METHOD_SERVICE);
                imm.restartInput(instance);
                imm.showSoftInput(instance, 0);
            }
        });
    }

    /**
     * Where the field's caret, selection and composition now are, in UTF-16 units
     * ({@code -1} for no composition) -- reported on every change, as an Android
     * editor reports it. A keyboard that predicts keeps its own model of the field,
     * and one never told of a change it did not make drifts from it: on a device it
     * wrote a word again beside itself (milestone 510).
     */
    public static void updateSelection(final Activity activity, final int selStart,
            final int selEnd, final int candStart, final int candEnd) {
        activity.runOnUiThread(new Runnable() {
            @Override
            public void run() {
                if (instance == null) {
                    return;
                }
                InputMethodManager imm =
                        (InputMethodManager) activity.getSystemService(Context.INPUT_METHOD_SERVICE);
                if (imm != null) {
                    imm.updateSelection(instance, selStart, selEnd, candStart, candEnd);
                }
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

    // --- Autofill (milestone 512) -------------------------------------------------

    /** One more field of the form being assembled; see {@link #publishFields}. */
    public static synchronized void addField(int id, String[] hints, int left, int top,
            int width, int height, String value, boolean sensitive) {
        pendingFields.add(new Field(id, hints, left, top, width, height, value, sensitive));
    }

    /** The form assembled since the last call becomes the one a service is shown. */
    public static synchronized void publishFields() {
        publishedFields = pendingFields;
        pendingFields = new ArrayList<Field>();
    }

    private static synchronized ArrayList<Field> currentFields() {
        return publishedFields;
    }

    private static Field findField(int id) {
        for (Field field : currentFields()) {
            if (field.id == id) {
                return field;
            }
        }
        return null;
    }

    private static AutofillManager autofillManager(Activity activity) {
        if (Build.VERSION.SDK_INT < 26 || instance == null) {
            return null;
        }
        return activity.getSystemService(AutofillManager.class);
    }

    /**
     * A field of the published form takes focus: the service is told where it is, which
     * is what lets it offer a saved value there. The box is the field's, moved from this
     * view's corner to the screen's.
     */
    public static void autofillEnter(final Activity activity, final int id) {
        activity.runOnUiThread(new Runnable() {
            @Override
            public void run() {
                AutofillManager afm = autofillManager(activity);
                Field field = findField(id);
                if (afm == null || field == null) {
                    return;
                }
                // The native surface fills the window, so a field's box is measured from
                // the window's corner -- not from this view's, which sits in the content
                // frame below the status bar. Seen on a device: offers anchored a status
                // bar too low (milestone 512).
                int[] origin = new int[2];
                instance.getRootView().getLocationOnScreen(origin);
                int left = origin[0] + field.left;
                int top = origin[1] + field.top;
                afm.notifyViewEntered(instance, id,
                        new Rect(left, top, left + field.width, top + field.height));
            }
        });
    }

    /** Focus leaves a field of the form. */
    public static void autofillExit(final Activity activity, final int id) {
        activity.runOnUiThread(new Runnable() {
            @Override
            public void run() {
                AutofillManager afm = autofillManager(activity);
                if (afm != null) {
                    afm.notifyViewExited(instance, id);
                }
            }
        });
    }

    /**
     * A field's value changed. A service keeps the values it was shown and saves those,
     * so a password typed after the form was published is saved as nothing unless it is
     * reported here.
     */
    public static void autofillValueChanged(final Activity activity, final int id,
            final String value) {
        activity.runOnUiThread(new Runnable() {
            @Override
            public void run() {
                AutofillManager afm = autofillManager(activity);
                if (afm != null) {
                    afm.notifyValueChanged(instance, id, AutofillValue.forText(value));
                }
            }
        });
    }

    /** The form is finished with: the service may offer to save what was typed in it. */
    public static void autofillCommit(final Activity activity) {
        activity.runOnUiThread(new Runnable() {
            @Override
            public void run() {
                AutofillManager afm = autofillManager(activity);
                if (afm != null) {
                    afm.commit();
                }
            }
        });
    }

    private FrusTextBridge(Context context) {
        super(context);
        setFocusable(true);
        setFocusableInTouchMode(true);
        if (Build.VERSION.SDK_INT >= 26) {
            // A 1x1 view is not somewhere a service looks for fields unless told to.
            setImportantForAutofill(IMPORTANT_FOR_AUTOFILL_YES);
        }
    }

    /** The published form, one virtual child per field, as the service asks for it. */
    @Override
    public void onProvideAutofillVirtualStructure(ViewStructure structure, int flags) {
        if (Build.VERSION.SDK_INT < 26) {
            return;
        }
        ArrayList<Field> form = currentFields();
        AutofillId parent = structure.getAutofillId();
        // The boxes arrive measured from the window's corner; a child's are this view's.
        int[] self = new int[2];
        int[] root = new int[2];
        getLocationOnScreen(self);
        getRootView().getLocationOnScreen(root);
        int dx = self[0] - root[0];
        int dy = self[1] - root[1];
        int index = structure.addChildCount(form.size());
        for (Field field : form) {
            ViewStructure child = structure.newChild(index++);
            child.setAutofillId(parent, field.id);
            if (field.hints.length > 0) {
                child.setAutofillHints(field.hints);
            }
            child.setAutofillType(View.AUTOFILL_TYPE_TEXT);
            child.setAutofillValue(AutofillValue.forText(field.value));
            child.setDataIsSensitive(field.sensitive);
            child.setVisibility(View.VISIBLE);
            child.setDimens(field.left - dx, field.top - dy, 0, 0, field.width, field.height);
        }
    }

    /** The values the service chose, each for the field it names, to the native side. */
    @Override
    public void autofill(SparseArray<AutofillValue> values) {
        if (Build.VERSION.SDK_INT < 26) {
            return;
        }
        for (int i = 0; i < values.size(); i++) {
            AutofillValue value = values.valueAt(i);
            if (value != null && value.isText()) {
                nativeAutofill(values.keyAt(i), value.getTextValue().toString());
            }
        }
    }

    @Override
    public boolean onCheckIsTextEditor() {
        return true;
    }

    @Override
    public InputConnection onCreateInputConnection(EditorInfo out) {
        // Whatever the focused field asked for. Composition and suggestions stay on
        // for the types that want them, the context being supplied by the get*Cursor
        // natives: this is what lights up the composition underline and the IME's
        // proposals -- and what a password type deliberately turns off.
        out.inputType = inputType;
        out.imeOptions = imeOptions;
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

        // A keyboard reclaiming a word it has finished, to go on predicting it: the
        // region is the field's, so it goes to the native side like the rest.
        @Override
        public boolean setComposingRegion(int start, int end) {
            nativeSetComposingRegion(start, end);
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
