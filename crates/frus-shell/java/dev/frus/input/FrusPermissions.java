package dev.frus.input;

import android.app.Activity;
import android.content.Context;
import android.content.Intent;
import android.content.SharedPreferences;
import android.content.pm.PackageManager;
import android.net.Uri;
import android.os.Build;
import android.provider.Settings;

/**
 * The runtime permissions an application asks the person for (milestone 730): the camera, the
 * microphone, the location, notifications. Called from Rust through JNI, as the other bridges
 * are; a NativeActivity cannot receive {@code onRequestPermissionsResult}, so the answer is
 * read from the permission itself, and a refusal from the question's dialog going away.
 */
public final class FrusPermissions {
    private FrusPermissions() {}

    /** Where the permissions this application has asked for before are remembered. */
    private static final String ASKED = "dev.frus.permissions.asked";

    /**
     * What the person has said about every one of {@code permissions}: 1 all granted; 0 not
     * asked yet, so asking shows the question; -1 refused, and asking again shows it again;
     * -2 refused for good ("don't ask again", or twice on Android 11 and later), so only the
     * application's settings can change it; -3 not in the application's manifest, which the
     * system refuses without asking.
     */
    public static int status(Activity activity, String[] permissions) {
        if (Build.VERSION.SDK_INT < 23) {
            // Granted at installation, by the manifest; one it does not name was never granted.
            for (String permission : permissions) {
                if (!declared(activity, permission)) {
                    return -3;
                }
            }
            return 1;
        }
        int worst = 1;
        for (String permission : permissions) {
            if (!declared(activity, permission)) {
                return -3;
            }
            if (activity.checkSelfPermission(permission) == PackageManager.PERMISSION_GRANTED) {
                continue;
            }
            int one;
            if (activity.shouldShowRequestPermissionRationale(permission)) {
                one = -1;
            } else {
                one = asked(activity, permission) ? -2 : 0;
            }
            worst = Math.min(worst, one);
        }
        return worst;
    }

    /** Whether the application's manifest asks for {@code permission}. */
    private static boolean declared(Activity activity, String permission) {
        try {
            String[] wanted = activity.getPackageManager()
                    .getPackageInfo(activity.getPackageName(), PackageManager.GET_PERMISSIONS)
                    .requestedPermissions;
            if (wanted != null) {
                for (String one : wanted) {
                    if (permission.equals(one)) {
                        return true;
                    }
                }
            }
        } catch (PackageManager.NameNotFoundException e) {
            // Its own package is always found.
        }
        return false;
    }

    private static boolean asked(Activity activity, String permission) {
        return activity.getSharedPreferences(ASKED, Context.MODE_PRIVATE)
                .getBoolean(permission, false);
    }

    /**
     * Whether the activity's window has the focus: lost while the question is up, given back
     * once it is answered -- the one sign of a refusal.
     */
    public static boolean focused(Activity activity) {
        return activity.hasWindowFocus();
    }

    /** Asks the person for {@code permissions}, and remembers that they were asked. */
    public static void ask(final Activity activity, final String[] permissions) {
        if (Build.VERSION.SDK_INT < 23) {
            return;
        }
        SharedPreferences.Editor asked =
                activity.getSharedPreferences(ASKED, Context.MODE_PRIVATE).edit();
        for (String permission : permissions) {
            asked.putBoolean(permission, true);
        }
        asked.apply();
        activity.runOnUiThread(new Runnable() {
            @Override
            public void run() {
                activity.requestPermissions(permissions, 0x7f02);
            }
        });
    }

    /** Opens the application's page in the system's settings, where a permission is changed. */
    public static void openSettings(Activity activity) {
        Intent intent = new Intent(Settings.ACTION_APPLICATION_DETAILS_SETTINGS,
                Uri.fromParts("package", activity.getPackageName(), null));
        intent.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK);
        activity.startActivity(intent);
    }
}
