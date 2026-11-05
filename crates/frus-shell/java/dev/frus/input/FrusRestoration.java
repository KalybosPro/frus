package dev.frus.input;

import android.app.Activity;
import android.app.Application;
import android.os.Build;
import android.os.Bundle;

/**
 * State restoration (milestone 753): what the application keeps goes into the activity's saved
 * instance state, which Android hands back when it starts the process again after ending it in
 * the background.
 *
 * <p>The bytes go under the key NativeActivity reads its own saved state from, so that the
 * next {@code onCreate} hands them to the native side with the rest of its creation. They are
 * written <em>after</em> the activity has saved its own state
 * ({@code onActivityPostSaveInstanceState}, Android 10 and later): NativeActivity writes the
 * same key from its native code, with what it was created with, and the latest state must be
 * the one that stays. Before Android 10 there is no such moment, and nothing is installed: an
 * application there starts afresh rather than from a state that may be stale.
 */
public final class FrusRestoration implements Application.ActivityLifecycleCallbacks {
    /** The key NativeActivity keeps its native saved state under. */
    private static final String KEY = "android:native_state";

    private FrusRestoration() {}

    /** What the application keeps now, or {@code null} for nothing. */
    private static native byte[] nativeSnapshot();

    /** Starts keeping the application's state with {@code activity}'s. Whether it could. */
    public static boolean install(Activity activity) {
        if (Build.VERSION.SDK_INT < 29) {
            return false;
        }
        activity.registerActivityLifecycleCallbacks(new FrusRestoration());
        return true;
    }

    @Override
    public void onActivityPostSaveInstanceState(Activity activity, Bundle outState) {
        byte[] state = nativeSnapshot();
        if (state != null) {
            outState.putByteArray(KEY, state);
        } else {
            outState.remove(KEY);
        }
    }

    @Override
    public void onActivityCreated(Activity activity, Bundle savedInstanceState) {}

    @Override
    public void onActivityStarted(Activity activity) {}

    @Override
    public void onActivityResumed(Activity activity) {}

    @Override
    public void onActivityPaused(Activity activity) {}

    @Override
    public void onActivityStopped(Activity activity) {}

    @Override
    public void onActivitySaveInstanceState(Activity activity, Bundle outState) {}

    @Override
    public void onActivityDestroyed(Activity activity) {}
}
