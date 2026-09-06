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
            Log.w(TAG, "libnilhal.so not bundled; running in pure hosted mobile mode: " + t.getMessage());
            isNativeLoaded = false;
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

    private static native void nativeSurfaceCreated(Surface surface);
    private static native void nativeSurfaceChanged(Surface surface, int width, int height);
    private static native void nativeSurfaceDestroyed();
    private static native void nativeHostTouchEvent(int action, int pointerId, float x, float y, float pressure);
    private static native void nativeHostKeyEvent(int action, int keycode, char character);
    private static native void nativeStartBridgeServer(String filesDirPath);
}
