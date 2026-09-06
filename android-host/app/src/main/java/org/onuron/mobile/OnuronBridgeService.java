package org.onuron.mobile;

import android.app.Notification;
import android.app.NotificationChannel;
import android.app.NotificationManager;
import android.app.Service;
import android.content.Context;
import android.content.Intent;
import android.net.ConnectivityManager;
import android.net.Network;
import android.net.NetworkCapabilities;
import android.os.BatteryManager;
import android.os.IBinder;
import android.telephony.TelephonyManager;

/**
 * OnuronBridgeService — Foreground Service managing Host Hardware Subsystems
 * Bridges Telephony, Battery, Wi-Fi, Audio, and Camera to Onuron userspace.
 */
public class OnuronBridgeService extends Service {

    private static final String CHANNEL_ID = "onuron_runtime_channel";

    @Override
    public void onCreate() {
        super.onCreate();
        createNotificationChannel();
        Notification notification = new Notification.Builder(this, CHANNEL_ID)
                .setContentTitle("Onuron OS Runtime")
                .setContentText("Hardware subsystem bridge active (Galaxy S25)")
                .setSmallIcon(android.R.drawable.stat_notify_sync)
                .build();
        startForeground(1001, notification);

        // Start background bridge listener thread
        NativeBridge.startBridgeServer(getFilesDir().getAbsolutePath());
    }

    private void createNotificationChannel() {
        NotificationChannel channel = new NotificationChannel(
                CHANNEL_ID,
                "Onuron OS Bridge",
                NotificationManager.IMPORTANCE_LOW
        );
        NotificationManager manager = getSystemService(NotificationManager.class);
        if (manager != null) {
            manager.createNotificationChannel(channel);
        }
    }

    @Override
    public int onStartCommand(Intent intent, int flags, int startId) {
        return START_STICKY;
    }

    @Override
    public IBinder onBind(Intent intent) {
        return null;
    }
}
