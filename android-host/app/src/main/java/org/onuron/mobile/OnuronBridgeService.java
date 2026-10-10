package org.onuron.mobile;

import android.app.Notification;
import android.app.NotificationChannel;
import android.app.NotificationManager;
import android.app.Service;
import android.content.BroadcastReceiver;
import android.content.Context;
import android.content.Intent;
import android.content.IntentFilter;
import android.hardware.camera2.CameraManager;
import android.media.AudioManager;
import android.net.ConnectivityManager;
import android.net.Network;
import android.net.NetworkCapabilities;
import android.net.NetworkRequest;
import android.net.Uri;
import android.os.BatteryManager;
import android.os.Build;
import android.os.IBinder;
import android.os.VibrationEffect;
import android.os.Vibrator;
import android.telephony.SmsManager;
import android.util.Log;

import org.json.JSONObject;

import java.net.InetAddress;
import java.net.NetworkInterface;
import java.util.Collections;

/**
 * OnuronBridgeService — Background Service managing Host Hardware Subsystems
 * Bridges Telephony, Battery, Wi-Fi, Audio, Notifications, and Camera to Onuron userspace.
 * Follows Milestone 2 security and privacy rules: redacted logging and validated command limits.
 */
public class OnuronBridgeService extends Service {

    private static final String TAG = "OnuronBridgeService";
    private static final String NOTIFICATION_CHANNEL_ID = "onuron_system_channel";

    private volatile boolean isRunning = false;
    private Thread commandDispatcherThread;

    private BroadcastReceiver batteryReceiver;
    private ConnectivityManager.NetworkCallback networkCallback;

    @Override
    public void onCreate() {
        super.onCreate();
        Log.i(TAG, "OnuronBridgeService initializing");

        createNotificationChannel();

        try {
            // Start background bridge listener thread if native layer is available
            NativeBridge.startBridgeServer(getFilesDir().getAbsolutePath());
        } catch (Throwable t) {
            Log.w(TAG, "Bridge server not started: " + t.getMessage());
        }

        registerTelemetryListeners();
        startCommandDispatcher();
    }

    private void createNotificationChannel() {
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
            NotificationChannel channel = new NotificationChannel(
                    NOTIFICATION_CHANNEL_ID,
                    "Onuron OS System",
                    NotificationManager.IMPORTANCE_DEFAULT
            );
            channel.setDescription("System notifications from Onuron OS");
            NotificationManager nm = (NotificationManager) getSystemService(Context.NOTIFICATION_SERVICE);
            if (nm != null) {
                nm.createNotificationChannel(channel);
            }
        }
    }

    private void registerTelemetryListeners() {
        // 1. Battery Telemetry Listener
        batteryReceiver = new BroadcastReceiver() {
            @Override
            public void onReceive(Context context, Intent intent) {
                int level = intent.getIntExtra(BatteryManager.EXTRA_LEVEL, -1);
                int scale = intent.getIntExtra(BatteryManager.EXTRA_SCALE, -1);
                int status = intent.getIntExtra(BatteryManager.EXTRA_STATUS, -1);
                int temp = intent.getIntExtra(BatteryManager.EXTRA_TEMPERATURE, -1);
                int volt = intent.getIntExtra(BatteryManager.EXTRA_VOLTAGE, -1);

                if (level >= 0 && scale > 0) {
                    int pct = (level * 100) / scale;
                    boolean isCharging = (status == BatteryManager.BATTERY_STATUS_CHARGING
                            || status == BatteryManager.BATTERY_STATUS_FULL);
                    float tempC = temp > 0 ? (temp / 10.0f) : 25.0f;
                    int voltMv = volt > 0 ? volt : 3800;

                    try {
                        JSONObject payload = new JSONObject();
                        payload.put("level", pct);
                        payload.put("is_charging", isCharging);
                        payload.put("temperature_c", tempC);
                        payload.put("voltage_mv", voltMv);

                        JSONObject event = new JSONObject();
                        event.put("type", "BatteryUpdate");
                        event.put("payload", payload);

                        NativeBridge.pushHostEventJson(event.toString());
                    } catch (Throwable t) {
                        Log.w(TAG, "Failed to push BatteryUpdate JSON", t);
                    }
                }
            }
        };
        registerReceiver(batteryReceiver, new IntentFilter(Intent.ACTION_BATTERY_CHANGED));

        // 2. Network Telemetry Listener
        try {
            ConnectivityManager cm = (ConnectivityManager) getSystemService(Context.CONNECTIVITY_SERVICE);
            if (cm != null) {
                NetworkRequest request = new NetworkRequest.Builder().build();
                networkCallback = new ConnectivityManager.NetworkCallback() {
                    @Override
                    public void onAvailable(Network network) {
                        pushNetworkTelemetry(cm, network, true);
                    }

                    @Override
                    public void onLost(Network network) {
                        pushNetworkTelemetry(cm, network, false);
                    }

                    @Override
                    public void onCapabilitiesChanged(Network network, NetworkCapabilities networkCapabilities) {
                        pushNetworkTelemetry(cm, network, true);
                    }
                };
                cm.registerNetworkCallback(request, networkCallback);
            }
        } catch (Throwable t) {
            Log.w(TAG, "Failed to register network callback", t);
        }
    }

    private void pushNetworkTelemetry(ConnectivityManager cm, Network network, boolean isConnected) {
        try {
            String connType = "NONE";
            if (isConnected && cm != null) {
                NetworkCapabilities caps = cm.getNetworkCapabilities(network);
                if (caps != null) {
                    if (caps.hasTransport(NetworkCapabilities.TRANSPORT_WIFI)) {
                        connType = "WIFI";
                    } else if (caps.hasTransport(NetworkCapabilities.TRANSPORT_CELLULAR)) {
                        connType = "CELLULAR_5G";
                    } else if (caps.hasTransport(NetworkCapabilities.TRANSPORT_ETHERNET)) {
                        connType = "ETHERNET";
                    }
                }
            }

            JSONObject payload = new JSONObject();
            payload.put("is_connected", isConnected);
            payload.put("conn_type", connType);
            payload.put("ip_address", isConnected ? getLocalIpAddress() : JSONObject.NULL);
            payload.put("ssid", JSONObject.NULL);

            JSONObject event = new JSONObject();
            event.put("type", "NetworkUpdate");
            event.put("payload", payload);

            NativeBridge.pushHostEventJson(event.toString());
        } catch (Throwable t) {
            Log.w(TAG, "Failed to push NetworkUpdate JSON", t);
        }
    }

    private String getLocalIpAddress() {
        try {
            for (NetworkInterface intf : Collections.list(NetworkInterface.getNetworkInterfaces())) {
                for (InetAddress addr : Collections.list(intf.getInetAddresses())) {
                    if (!addr.isLoopbackAddress() && addr.getAddress().length == 4) {
                        return addr.getHostAddress();
                    }
                }
            }
        } catch (Throwable ignored) {}
        return null;
    }

    private synchronized void startCommandDispatcher() {
        if (commandDispatcherThread != null && commandDispatcherThread.isAlive()) {
            Log.i(TAG, "Command dispatcher worker thread already active");
            return;
        }
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

    private static String redactNumber(String num) {
        if (num == null || num.isEmpty()) return "***";
        if (num.length() <= 4) return "***";
        return num.substring(0, Math.min(2, num.length())) + "***" + num.substring(num.length() - 2);
    }

    private void handleGuestCommand(String json) {
        try {
            JSONObject obj = new JSONObject(json);
            String action = obj.optString("action");
            JSONObject params = obj.optJSONObject("params");

            if ("DialNumber".equals(action) && params != null) {
                String number = params.optString("number").trim();
                // Validate digits/plus/hash
                if (number.matches("^[+*#0-9]{1,32}$")) {
                    Log.i(TAG, "Executing dialer intent for: " + redactNumber(number));
                    Intent callIntent = new Intent(Intent.ACTION_DIAL);
                    callIntent.setData(Uri.parse("tel:" + number));
                    callIntent.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK);
                    startActivity(callIntent);
                } else {
                    Log.w(TAG, "DialNumber rejected: invalid number format or length");
                }
            } else if ("SendSms".equals(action) && params != null) {
                String recipient = params.optString("recipient").trim();
                String message = params.optString("message");
                if (recipient.matches("^[+*#0-9]{1,32}$") && message != null && !message.isEmpty() && message.length() <= 1600) {
                    Log.i(TAG, "Sending SMS via host bridge to: " + redactNumber(recipient));
                    try {
                        SmsManager sms = SmsManager.getDefault();
                        sms.sendTextMessage(recipient, null, message, null, null);
                        Log.i(TAG, "SMS successfully dispatched via SmsManager");
                    } catch (Throwable t) {
                        Log.w(TAG, "SmsManager send failed; fallback to intent", t);
                        Intent smsIntent = new Intent(Intent.ACTION_SENDTO);
                        smsIntent.setData(Uri.parse("smsto:" + recipient));
                        smsIntent.putExtra("sms_body", message);
                        smsIntent.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK);
                        startActivity(smsIntent);
                    }
                } else {
                    Log.w(TAG, "SendSms rejected: invalid recipient format or message bounds");
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
                                Log.i(TAG, "Torch mode set to " + enabled);
                            }
                        }
                    } catch (Throwable t) {
                        Log.e(TAG, "Torch control error", t);
                    }
                }
            } else if ("Vibrate".equals(action) && params != null) {
                long duration = Math.min(params.optLong("duration_ms", 100), 5000); // Max 5s guard
                Vibrator v = (Vibrator) getSystemService(Context.VIBRATOR_SERVICE);
                if (v != null) {
                    if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
                        v.vibrate(VibrationEffect.createOneShot(duration, VibrationEffect.DEFAULT_AMPLITUDE));
                    } else {
                        v.vibrate(duration);
                    }
                }
            } else if ("SetVolume".equals(action) && params != null) {
                int percent = Math.max(0, Math.min(100, params.optInt("percent", 50)));
                AudioManager am = (AudioManager) getSystemService(Context.AUDIO_SERVICE);
                if (am != null) {
                    int maxVol = am.getStreamMaxVolume(AudioManager.STREAM_MUSIC);
                    int targetVol = (percent * maxVol) / 100;
                    am.setStreamVolume(AudioManager.STREAM_MUSIC, targetVol, 0);
                    Log.i(TAG, "Volume set to " + percent + "% (" + targetVol + "/" + maxVol + ")");
                }
            } else if ("PostSystemNotification".equals(action) && params != null) {
                int id = params.optInt("id", 1);
                String title = params.optString("title", "Onuron OS");
                String content = params.optString("content", "");

                NotificationManager nm = (NotificationManager) getSystemService(Context.NOTIFICATION_SERVICE);
                if (nm != null) {
                    Notification.Builder builder;
                    if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
                        builder = new Notification.Builder(this, NOTIFICATION_CHANNEL_ID);
                    } else {
                        builder = new Notification.Builder(this);
                    }
                    builder.setContentTitle(title)
                            .setContentText(content)
                            .setSmallIcon(android.R.drawable.stat_notify_more)
                            .setAutoCancel(true);
                    nm.notify(id, builder.build());
                    Log.i(TAG, "Notification posted: " + title);
                }
            } else {
                Log.w(TAG, "Unhandled or unrecognised guest command action: " + action);
            }
        } catch (Throwable t) {
            Log.e(TAG, "Failed to parse guest command JSON (payload redacted for privacy)", t);
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
            try {
                commandDispatcherThread.join(500);
            } catch (InterruptedException ignored) {}
            commandDispatcherThread = null;
        }

        if (batteryReceiver != null) {
            try {
                unregisterReceiver(batteryReceiver);
            } catch (Throwable ignored) {}
            batteryReceiver = null;
        }

        if (networkCallback != null) {
            try {
                ConnectivityManager cm = (ConnectivityManager) getSystemService(Context.CONNECTIVITY_SERVICE);
                if (cm != null) {
                    cm.unregisterNetworkCallback(networkCallback);
                }
            } catch (Throwable ignored) {}
            networkCallback = null;
        }

        super.onDestroy();
        Log.i(TAG, "OnuronBridgeService destroyed");
    }

    @Override
    public IBinder onBind(Intent intent) {
        return null;
    }
}
