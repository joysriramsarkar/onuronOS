package org.onuron.mobile;

import android.app.Service;
import android.content.Intent;
import android.os.IBinder;
import android.util.Log;

/**
 * OnuronBridgeService — Background Service managing Host Hardware Subsystems
 * Bridges Telephony, Battery, Wi-Fi, Audio, and Camera to Onuron userspace.
 */
public class OnuronBridgeService extends Service {

    private static final String TAG = "OnuronBridgeService";

    @Override
    public void onCreate() {
        super.onCreate();
        Log.i(TAG, "OnuronBridgeService initialized");
        try {
            // Start background bridge listener thread if native layer is available
            NativeBridge.startBridgeServer(getFilesDir().getAbsolutePath());
        } catch (Throwable t) {
            Log.w(TAG, "Bridge server not started: " + t.getMessage());
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
