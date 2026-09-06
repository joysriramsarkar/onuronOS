package org.onuron.mobile;

import android.view.Surface;

/**
 * NativeBridge — JNI Bridge to Onuron OS C/Rust Runtime
 */
public class NativeBridge {

    static {
        System.loadLibrary("nilhal");
    }

    public static native void onSurfaceCreated(Surface surface);
    public static native void onSurfaceChanged(Surface surface, int width, int height);
    public static native void onSurfaceDestroyed();

    public static native void onHostTouchEvent(int action, int pointerId, float x, float y, float pressure);
    public static native void onHostKeyEvent(int action, int keycode, char character);

    public static native void startBridgeServer(String filesDirPath);
}
