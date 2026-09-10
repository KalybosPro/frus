package dev.frus.input;

import android.app.Activity;
import android.os.Build;
import android.view.View;
import android.view.Window;
import android.view.WindowInsetsController;
import android.view.WindowManager;

/**
 * The system bars' colour and icons (issue #46), set on the Java UI thread: the
 * Window and DecorView calls below check the thread they are made on, and the
 * native code runs on a thread of its own.
 *
 * <p>A pipe, not a policy: which colour and which icons is decided on the Rust
 * side, so that a new rule never needs a new dex. Shipped in the same dex as the
 * input bridge and loaded by the same class loader.
 */
public final class FrusSystemBars {
    private FrusSystemBars() {}

    /**
     * Paints both bars and chooses their icons.
     *
     * @param statusColor the status bar's colour, 0xAARRGGBB
     * @param navColor the navigation bar's colour, 0xAARRGGBB
     * @param darkStatusIcons dark icons on the status bar, for a light bar
     * @param darkNavIcons dark icons on the navigation bar
     */
    public static void apply(final Activity activity, final int statusColor, final int navColor,
            final boolean darkStatusIcons, final boolean darkNavIcons) {
        activity.runOnUiThread(new Runnable() {
            @Override
            public void run() {
                Window window = activity.getWindow();
                if (window == null) {
                    return;
                }
                // The colours are honoured only once the window draws the bars'
                // backgrounds. It is asked for here, once the first frame is up, and
                // not in the launch theme, where it brought the white flash back
                // (see the demo's res/values/styles.xml).
                window.clearFlags(WindowManager.LayoutParams.FLAG_TRANSLUCENT_STATUS
                        | WindowManager.LayoutParams.FLAG_TRANSLUCENT_NAVIGATION);
                window.addFlags(WindowManager.LayoutParams.FLAG_DRAWS_SYSTEM_BAR_BACKGROUNDS);
                window.setStatusBarColor(statusColor);
                window.setNavigationBarColor(navColor);

                // A "light" bar is the platform's word for a bar that wants dark icons.
                if (Build.VERSION.SDK_INT >= 30) {
                    WindowInsetsController controller = window.getInsetsController();
                    if (controller != null) {
                        int both = WindowInsetsController.APPEARANCE_LIGHT_STATUS_BARS
                                | WindowInsetsController.APPEARANCE_LIGHT_NAVIGATION_BARS;
                        int wanted = (darkStatusIcons
                                        ? WindowInsetsController.APPEARANCE_LIGHT_STATUS_BARS
                                        : 0)
                                | (darkNavIcons
                                        ? WindowInsetsController.APPEARANCE_LIGHT_NAVIGATION_BARS
                                        : 0);
                        controller.setSystemBarsAppearance(wanted, both);
                    }
                } else {
                    View decor = window.getDecorView();
                    int flags = decor.getSystemUiVisibility();
                    flags = darkStatusIcons
                            ? flags | View.SYSTEM_UI_FLAG_LIGHT_STATUS_BAR
                            : flags & ~View.SYSTEM_UI_FLAG_LIGHT_STATUS_BAR;
                    if (Build.VERSION.SDK_INT >= 26) {
                        flags = darkNavIcons
                                ? flags | View.SYSTEM_UI_FLAG_LIGHT_NAVIGATION_BAR
                                : flags & ~View.SYSTEM_UI_FLAG_LIGHT_NAVIGATION_BAR;
                    }
                    decor.setSystemUiVisibility(flags);
                }
            }
        });
    }
}
