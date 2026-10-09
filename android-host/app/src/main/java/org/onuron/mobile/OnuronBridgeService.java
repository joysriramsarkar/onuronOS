package org.onuron.mobile;

import android.app.Service;
import android.content.Context;
import android.content.Intent;
import android.hardware.camera2.CameraManager;
import android.net.Uri;
import android.os.Build;
import android.os.IBinder;
import android.os.Vibrator;
import android.telephony.SmsManager;
import android.util.Log;

import org.json.JSONObject;

/**
 * OnuronBridgeService — Background Service managing Host Hardware Subsystems
 * Bridges Telephony, Battery, Wi-Fi, Audio, and Camera to Onuron userspace.
 */
public class OnuronBridgeService extends Service {

    private static final String TAG = "OnuronBridgeService";
    private volatile boolean isRunning = false;
    private Thread commandDispatcherThread;

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

        startCommandDispatcher();
    }

    private void startCommandDispatcher() {
        isRunning = true;
        commandDispatcherThread = new Thread(() -> {
            Log.i(TAG, "Command dispatcher worker thread active");
            while (isRunning) {
                try {
                    String cmdJson = NativeBridge.pollGuestCommand();
                    if (cmdJson != null && !cmdJson.trim().isEmpty()) {
                        handleGuestCommand(cmdJson.trim());
                    } else {
                        Thread.sleep(50); // Small interval when queue is idle
                    }
                } catch (InterruptedException e) {
                    break;
                } catch (Throwable t) {
                    Log.e(TAG, "Error in command dispatcher loop", t);
                }
            }
        });
        commandDispatcherThread.setName("Onuron-Bridge-Dispatcher");
        commandDispatcherThread.start();
    }

    private void handleGuestCommand(String json) {
        try {
            JSONObject obj = new JSONObject(json);
            String action = obj.optString("action");
            JSONObject params = obj.optJSONObject("params");

            Log.i(TAG, "Dispatching command from Onuron: " + action);

            if ("DialNumber".equals(action) && params != null) {
                String number = params.optString("number");
                if (!number.isEmpty()) {
                    Intent callIntent = new Intent(Intent.ACTION_DIAL);
                    callIntent.setData(Uri.parse("tel:" + number));
                    callIntent.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK);
                    startActivity(callIntent);
                }
            } else if ("SendSms".equals(action) && params != null) {
                String recipient = params.optString("recipient");
                String message = params.optString("message");
                if (!recipient.isEmpty() && !message.isEmpty()) {
                    try {
                        SmsManager sms = SmsManager.getDefault();
                        sms.sendTextMessage(recipient, null, message, null, null);
                        Log.i(TAG, "SMS successfully sent via host bridge to " + recipient);
                    } catch (Throwable t) {
                        Log.w(TAG, "SmsManager send failed; fallback to intent", t);
                        Intent smsIntent = new Intent(Intent.ACTION_SENDTO);
                        smsIntent.setData(Uri.parse("smsto:" + recipient));
                        smsIntent.putExtra("sms_body", message);
                        smsIntent.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK);
                        startActivity(smsIntent);
                    }
                }
            } else if ("SetTorch".equals(action) && params != null) {
                boolean enabled = params.optBoolean("enabled", false);
                if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.M) {
                    try {
                        CameraManager cm = (CameraManager) getSystemService(Context.CAMERA_SERVICE);
                        if (cm != null) {
                            String[] camIds = cm.getCameraIdList();
                            if (camIds.length > 0) {
                                cm.setTorchMode(camIds[0], enabled);
                            }
                        }
                    } catch (Throwable t) {
                        Log.e(TAG, "Torch control error", t);
                    }
                }
            } else if ("Vibrate".equals(action) && params != null) {
                long duration = params.optLong("duration_ms", 100);
                Vibrator v = (Vibrator) getSystemService(Context.VIBRATOR_SERVICE);
                if (v != null) {
                    v.vibrate(duration);
                }
            }
        } catch (Throwable t) {
            Log.e(TAG, "Failed to parse guest command JSON: " + json, t);
        }
    }

    @Override
    public int onStartCommand(Intent intent, int flags, int startId) {
        return START_STICKY;
    }

    @Override
    public void onDestroy() {
        isRunning = false;
        if (commandDispatcherThread != null) {
            commandDispatcherThread.interrupt();
        }
        super.onDestroy();
    }

    @Override
    public IBinder onBind(Intent intent) {
        return null;
    }
}
