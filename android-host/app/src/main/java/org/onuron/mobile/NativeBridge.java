package org.onuron.mobile;

import android.util.Log;
import android.view.Surface;

/**
 * NativeBridge — JNI Bridge to Onuron OS C/Rust Runtime
 * Safely handles missing shared libraries without crashing.
 */
public class NativeBridge {

    private static final String TAG = "OnuronNativeBridge";
    public static boolean isNativeLoaded = false;

    static {
        try {
            System.loadLibrary("nilhal");
            isNativeLoaded = true;
            Log.i(TAG, "libnilhal.so loaded successfully");
        } catch (Throwable t) {
            try {
                System.loadLibrary("android_host");
                isNativeLoaded = true;
                Log.i(TAG, "libandroid_host.so loaded successfully");
            } catch (Throwable t2) {
                Log.w(TAG, "Native library not bundled; running in pure hosted mobile mode: " + t2.getMessage());
                isNativeLoaded = false;
            }
        }
    }

    public static void onSurfaceCreated(Surface surface) {
        if (isNativeLoaded) {
            try {
                nativeSurfaceCreated(surface);
            } catch (Throwable t) {
                Log.e(TAG, "nativeSurfaceCreated failed", t);
            }
        }
    }

    public static void onSurfaceChanged(Surface surface, int width, int height) {
        if (isNativeLoaded) {
            try {
                nativeSurfaceChanged(surface, width, height);
            } catch (Throwable t) {
                Log.e(TAG, "nativeSurfaceChanged failed", t);
            }
        }
    }

    public static void onSurfaceDestroyed() {
        if (isNativeLoaded) {
            try {
                nativeSurfaceDestroyed();
            } catch (Throwable t) {
                Log.e(TAG, "nativeSurfaceDestroyed failed", t);
            }
        }
    }

    public static void onHostTouchEvent(int action, int pointerId, float x, float y, float pressure) {
        if (isNativeLoaded) {
            try {
                nativeHostTouchEvent(action, pointerId, x, y, pressure);
            } catch (Throwable t) {
                Log.e(TAG, "nativeHostTouchEvent failed", t);
            }
        }
    }

    public static void onHostKeyEvent(int action, int keycode, char character) {
        if (isNativeLoaded) {
            try {
                nativeHostKeyEvent(action, keycode, character);
            } catch (Throwable t) {
                Log.e(TAG, "nativeHostKeyEvent failed", t);
            }
        }
    }

    public static void startBridgeServer(String filesDirPath) {
        if (isNativeLoaded) {
            try {
                nativeStartBridgeServer(filesDirPath);
            } catch (Throwable t) {
                Log.e(TAG, "nativeStartBridgeServer failed", t);
            }
        }
    }

    public static String pollGuestCommand() {
        if (isNativeLoaded) {
            try {
                return nativePollGuestCommand();
            } catch (Throwable t) {
                Log.e(TAG, "nativePollGuestCommand failed", t);
            }
        }
        return null;
    }

    public static void pushCameraFrame(byte[] buffer) {
        if (isNativeLoaded && buffer != null) {
            try {
                nativePushCameraFrame(buffer, buffer.length);
            } catch (Throwable t) {
                Log.e(TAG, "nativePushCameraFrame failed", t);
            }
        }
    }

    public static int pullAudioSamples(short[] outBuffer) {
        if (isNativeLoaded && outBuffer != null) {
            try {
                return nativePullAudioSamples(outBuffer, outBuffer.length);
            } catch (Throwable t) {
                Log.e(TAG, "nativePullAudioSamples failed", t);
            }
        }
        return 0;
    }

    public static void pushAudioSamples(short[] inBuffer) {
        if (isNativeLoaded && inBuffer != null) {
            try {
                nativePushAudioSamples(inBuffer, inBuffer.length);
            } catch (Throwable t) {
                Log.e(TAG, "nativePushAudioSamples failed", t);
            }
        }
    }

    public static int getLatestFrame(int[] outPixels) {
        if (isNativeLoaded && outPixels != null) {
            try {
                return nativeGetLatestFrame(outPixels, outPixels.length);
            } catch (Throwable t) {
                Log.e(TAG, "nativeGetLatestFrame failed", t);
            }
        }
        return 0;
    }

    public static boolean pushHostEventJson(String json) {
        if (isNativeLoaded && json != null) {
            try {
                return nativePushHostEventJson(json);
            } catch (Throwable t) {
                Log.e(TAG, "nativePushHostEventJson failed", t);
            }
        }
        return false;
    }

    private static native void nativeSurfaceCreated(Surface surface);
    private static native void nativeSurfaceChanged(Surface surface, int width, int height);
    private static native void nativeSurfaceDestroyed();
    private static native void nativeHostTouchEvent(int action, int pointerId, float x, float y, float pressure);
    private static native void nativeHostKeyEvent(int action, int keycode, char character);
    private static native void nativeStartBridgeServer(String filesDirPath);
    private static native String nativePollGuestCommand();
    private static native void nativePushCameraFrame(byte[] buffer, int len);
    private static native int nativePullAudioSamples(short[] outBuffer, int maxLen);
    private static native void nativePushAudioSamples(short[] inBuffer, int len);
    private static native int nativeGetLatestFrame(int[] outPixels, int maxLen);
    private static native boolean nativePushHostEventJson(String json);
}
