package dev.frus.input;

import android.app.Activity;
import android.content.ClipData;
import android.content.ClipboardManager;
import android.content.Context;

/**
 * The system clipboard (issue #22), read and written for the native side.
 *
 * <p>Neither call needs the UI thread: the clipboard service is a binder call, and
 * the manager is built on the main thread's handler whichever thread asks for it.
 * The read has to be synchronous anyway — a paste wants its text now.
 *
 * <p>From Android 12 the platform shows a toast when an application reads the
 * clipboard. That is the platform telling the user something true, and nothing
 * here tries to hide it.
 */
public final class FrusClipboard {
    private FrusClipboard() {}

    /**
     * The clipboard's text, or {@code null} when it holds nothing, or nothing that
     * reads as text. A clip that is not plain text — a link, a URI — is asked for
     * its text form, which is what a paste into a text field means.
     */
    public static String getText(Activity activity) {
        ClipboardManager manager =
                (ClipboardManager) activity.getSystemService(Context.CLIPBOARD_SERVICE);
        if (manager == null) {
            return null;
        }
        ClipData clip = manager.getPrimaryClip();
        if (clip == null || clip.getItemCount() == 0) {
            return null;
        }
        CharSequence text = clip.getItemAt(0).coerceToText(activity);
        if (text == null || text.length() == 0) {
            return null;
        }
        return text.toString();
    }

    /** Puts {@code text} on the clipboard, as plain text. */
    public static void setText(Activity activity, String text) {
        ClipboardManager manager =
                (ClipboardManager) activity.getSystemService(Context.CLIPBOARD_SERVICE);
        if (manager == null) {
            return;
        }
        manager.setPrimaryClip(ClipData.newPlainText("text", text));
    }
}
