package dev.frus.input;

import android.app.Activity;
import android.content.Context;
import android.text.InputType;
import android.view.KeyEvent;
import android.view.View;
import android.view.ViewGroup;
import android.view.inputmethod.BaseInputConnection;
import android.view.inputmethod.EditorInfo;
import android.view.inputmethod.InputConnection;
import android.view.inputmethod.InputMethodManager;

/**
 * Pont de saisie frus (voir docs/jalon-81.md) : NativeActivity n'offre aucune
 * InputConnection, donc les IME tournent en mode dégradé (TYPE_NULL, touches
 * latines seulement — ni composition, ni swipe, ni CJK). Cette View 1×1
 * focalisable, ajoutée par-dessus le contenu natif, fournit une vraie
 * InputConnection et relaie chaque opération de l'IME au code natif
 * (méthodes `native*`, enregistrées via RegisterNatives côté Rust).
 *
 * Compilée en dex embarqué (scripts/build-input-dex.sh), chargée à l'exécution
 * par InMemoryDexClassLoader — aucun changement de packaging.
 */
public final class FrusTextBridge extends View {
    private static native void nativeCommit(String text);
    private static native void nativeSetComposing(String text);
    private static native void nativeFinishComposing();
    private static native void nativeDelete(int before, int after);
    private static native void nativeEditorAction(int action);
    /** Renvoie true si le natif a consommé la touche (édition). */
    private static native boolean nativeKey(int keyCode, boolean down, int unicode, int meta);

    private static FrusTextBridge instance;

    /** Ajoute la view pont à l'activité (une seule fois). */
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

    /** Un champ texte natif prend le focus : la view pont capte l'IME. */
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

    /** Le focus quitte les champs texte : IME refermé, focus relâché. */
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
        out.inputType = InputType.TYPE_CLASS_TEXT | InputType.TYPE_TEXT_FLAG_NO_SUGGESTIONS;
        out.imeOptions = EditorInfo.IME_ACTION_DONE | EditorInfo.IME_FLAG_NO_FULLSCREEN;
        return new Connection(this);
    }

    // Touches physiques/dpad reçues par la view focalisée : l'édition est
    // consommée côté natif ; le reste (Retour…) suit le chemin par défaut
    // jusqu'à NativeActivity (geste retour, navigation).
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

    /** L'InputConnection réelle : chaque opération de l'IME part au natif. */
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

        @Override
        public boolean sendKeyEvent(KeyEvent event) {
            // Certains IME envoient Effacer/Entrée par ici plutôt que par
            // deleteSurroundingText/performEditorAction.
            return nativeKey(
                    event.getKeyCode(),
                    event.getAction() == KeyEvent.ACTION_DOWN,
                    event.getUnicodeChar(),
                    event.getMetaState());
        }
    }
}
