package dev.frus.input;

import android.app.Activity;
import android.app.Notification;
import android.app.NotificationChannel;
import android.app.NotificationManager;
import android.app.PendingIntent;
import android.content.BroadcastReceiver;
import android.content.Context;
import android.content.Intent;
import android.content.IntentFilter;
import android.content.pm.PackageManager;
import android.graphics.Bitmap;
import android.graphics.BitmapFactory;
import android.os.Build;
import android.service.notification.StatusBarNotification;

import java.util.concurrent.ConcurrentHashMap;
import java.util.concurrent.atomic.AtomicInteger;

/**
 * The system's notifications (milestone 703), posted for the native side and answered back
 * to it.
 *
 * <p>A click on a notification opens the application through its launch intent, directly:
 * since Android 12 a notification may not start an activity from a broadcast. Its buttons and
 * its dismissal are broadcasts to a receiver registered here, which records the answer for the
 * native side to collect. A notification that has left the shade with no answer recorded was
 * therefore clicked: the dismissal has its own broadcast, and a click does not.
 *
 * <p>No call needs the UI thread but {@link #ask}, which runs itself there.
 */
public final class FrusNotifications extends BroadcastReceiver {
    private static final String CHANNEL = "frus.notifications";
    private static final String QUIET_CHANNEL = "frus.notifications.silent";
    private static final String PERMISSION = "android.permission.POST_NOTIFICATIONS";
    /** What a button or a dismissal answers; read by the native side. */
    private static final ConcurrentHashMap<Integer, String> ANSWERS = new ConcurrentHashMap<>();
    private static final AtomicInteger NEXT = new AtomicInteger(1);
    private static FrusNotifications receiver;

    private FrusNotifications() {}

    private static String action(Context context) {
        return context.getPackageName() + ".frus.NOTIFICATION";
    }

    /** Registers the receiver of the answers, once. */
    private static synchronized void listen(Activity activity) {
        if (receiver != null) {
            return;
        }
        receiver = new FrusNotifications();
        Context app = activity.getApplicationContext();
        IntentFilter filter = new IntentFilter(action(app));
        if (Build.VERSION.SDK_INT >= 33) {
            app.registerReceiver(receiver, filter, Context.RECEIVER_NOT_EXPORTED);
        } else {
            app.registerReceiver(receiver, filter);
        }
    }

    @Override
    public void onReceive(Context context, Intent intent) {
        int id = intent.getIntExtra("id", 0);
        String answer = intent.getStringExtra("answer");
        if (id == 0 || answer == null) {
            return;
        }
        ANSWERS.put(id, answer);
        if (!answer.equals("closed")) {
            // A button leaves the notification in the shade unless it is taken away.
            NotificationManager manager =
                    (NotificationManager) context.getSystemService(Context.NOTIFICATION_SERVICE);
            if (manager != null) {
                manager.cancel(id);
            }
        }
    }

    /**
     * Whether notifications may be shown: 1 they may, 0 the person has not said (on Android 13
     * and later, until they answer the question {@link #ask} puts), -1 they may not — turned off
     * for the application, or the permission is missing from its manifest.
     */
    public static int allowed(Activity activity) {
        NotificationManager manager =
                (NotificationManager) activity.getSystemService(Context.NOTIFICATION_SERVICE);
        if (manager == null) {
            return -1;
        }
        if (Build.VERSION.SDK_INT >= 33
                && activity.checkSelfPermission(PERMISSION) != PackageManager.PERMISSION_GRANTED) {
            return declared(activity) ? 0 : -1;
        }
        return manager.areNotificationsEnabled() ? 1 : -1;
    }

    /** Whether the application's manifest asks for the permission to post notifications. */
    private static boolean declared(Activity activity) {
        try {
            String[] wanted = activity.getPackageManager()
                    .getPackageInfo(activity.getPackageName(), PackageManager.GET_PERMISSIONS)
                    .requestedPermissions;
            if (wanted != null) {
                for (String permission : wanted) {
                    if (PERMISSION.equals(permission)) {
                        return true;
                    }
                }
            }
        } catch (PackageManager.NameNotFoundException e) {
            // Its own package is always found; nothing to do.
        }
        return false;
    }

    /** Asks the person for the permission to post notifications (Android 13 and later). */
    public static void ask(final Activity activity) {
        if (Build.VERSION.SDK_INT < 33) {
            return;
        }
        activity.runOnUiThread(new Runnable() {
            @Override
            public void run() {
                activity.requestPermissions(new String[] {PERMISSION}, 0x7f01);
            }
        });
    }

    /**
     * Posts a notification; its number, which {@link #active} and {@link #answer} take, or 0
     * when it could not be.
     */
    @SuppressWarnings("deprecation")
    public static int post(Activity activity, String title, String body, String[] buttonIds,
            String[] buttonLabels, String image, boolean silent) {
        listen(activity);
        Context app = activity.getApplicationContext();
        NotificationManager manager =
                (NotificationManager) app.getSystemService(Context.NOTIFICATION_SERVICE);
        if (manager == null) {
            return 0;
        }
        int id = NEXT.getAndIncrement();
        Notification.Builder builder;
        if (Build.VERSION.SDK_INT >= 26) {
            String channel = silent ? QUIET_CHANNEL : CHANNEL;
            if (manager.getNotificationChannel(channel) == null) {
                NotificationChannel made = silent
                        ? new NotificationChannel(channel, "Silent notifications",
                                NotificationManager.IMPORTANCE_LOW)
                        : new NotificationChannel(channel, "Notifications",
                                NotificationManager.IMPORTANCE_DEFAULT);
                manager.createNotificationChannel(made);
            }
            builder = new Notification.Builder(app, channel);
        } else {
            builder = new Notification.Builder(app);
            if (!silent) {
                builder.setDefaults(Notification.DEFAULT_ALL);
            }
        }
        builder.setSmallIcon(app.getApplicationInfo().icon)
                .setContentTitle(title)
                .setAutoCancel(true)
                .setDeleteIntent(answering(app, id, 0, "closed"));
        if (body != null && !body.isEmpty()) {
            builder.setContentText(body);
        }
        Intent launch = app.getPackageManager().getLaunchIntentForPackage(app.getPackageName());
        if (launch != null) {
            // Into the running activity, never beside it: a second NativeActivity in the same
            // process cannot start (its event loop cannot be made twice). The launcher's own
            // intent only reaches the running one when the application was started from the
            // launcher; these flags make it do so however it was started.
            launch.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK | Intent.FLAG_ACTIVITY_SINGLE_TOP);
            builder.setContentIntent(PendingIntent.getActivity(app, id * 16, launch, flags()));
        }
        for (int i = 0; i < buttonIds.length && i < 3; i++) {
            builder.addAction(new Notification.Action.Builder(0, buttonLabels[i],
                    answering(app, id, i + 1, "button:" + buttonIds[i])).build());
        }
        if (image != null) {
            Bitmap picture = BitmapFactory.decodeFile(image);
            if (picture != null) {
                builder.setLargeIcon(picture)
                        .setStyle(new Notification.BigPictureStyle().bigPicture(picture));
            }
        }
        manager.notify(id, builder.build());
        return id;
    }

    /** A broadcast to the receiver that records `answer` for notification `id`. */
    private static PendingIntent answering(Context app, int id, int slot, String answer) {
        Intent intent = new Intent(action(app))
                .setPackage(app.getPackageName())
                .putExtra("id", id)
                .putExtra("answer", answer);
        return PendingIntent.getBroadcast(app, id * 16 + slot, intent, flags());
    }

    /**
     * Each intent replaces any left from an earlier run: numbers start again at 1 in every
     * process, and an intent only updated keeps its old flags — a leftover from before the
     * flags above opened a second activity.
     */
    private static int flags() {
        return PendingIntent.FLAG_CANCEL_CURRENT
                | (Build.VERSION.SDK_INT >= 23 ? PendingIntent.FLAG_IMMUTABLE : 0);
    }

    /** Whether notification `id` is still in the shade. */
    public static boolean active(Activity activity, int id) {
        NotificationManager manager =
                (NotificationManager) activity.getSystemService(Context.NOTIFICATION_SERVICE);
        if (manager == null) {
            return false;
        }
        for (StatusBarNotification shown : manager.getActiveNotifications()) {
            if (shown.getId() == id) {
                return true;
            }
        }
        return false;
    }

    /** The answer recorded for notification `id`, taken once; {@code null} when none is. */
    public static String answer(int id) {
        return ANSWERS.remove(id);
    }
}
