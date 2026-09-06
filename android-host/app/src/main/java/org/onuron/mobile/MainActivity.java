package org.onuron.mobile;

import android.app.Activity;
import android.content.Intent;
import android.os.Bundle;
import android.view.MotionEvent;
import android.view.SurfaceHolder;
import android.view.SurfaceView;
import android.view.View;
import android.view.WindowManager;

/**
 * MainActivity — Host Window for Onuron OS on Samsung Galaxy S25
 * Hosts the Native ANativeWindow surface and feeds touch/key events to Onuron's inputd.
 */
public class MainActivity extends Activity implements SurfaceHolder.Callback {

    private SurfaceView surfaceView;

    @Override
    protected void onCreate(Bundle savedInstanceState) {
        super.onCreate(savedInstanceState);

        // Keep screen on & immersive fullscreen
        getWindow().addFlags(WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON);
        getWindow().getDecorView().setSystemUiVisibility(
                View.SYSTEM_UI_FLAG_LAYOUT_STABLE
                        | View.SYSTEM_UI_FLAG_LAYOUT_HIDE_NAVIGATION
                        | View.SYSTEM_UI_FLAG_LAYOUT_FULLSCREEN
                        | View.SYSTEM_UI_FLAG_HIDE_NAVIGATION
                        | View.SYSTEM_UI_FLAG_FULLSCREEN
                        | View.SYSTEM_UI_FLAG_IMMERSIVE_STICKY);

        surfaceView = new SurfaceView(this);
        surfaceView.getHolder().addCallback(this);
        setContentView(surfaceView);

        // Start Onuron Background Bridge Daemon
        Intent serviceIntent = new Intent(this, OnuronBridgeService.class);
        startForegroundService(serviceIntent);
    }

    @Override
    public boolean onTouchEvent(MotionEvent event) {
        // Forward touch down/move/up events to Onuron Native Runtime
        int action = event.getActionMasked();
        int pointerCount = event.getPointerCount();

        for (int i = 0; i < pointerCount; i++) {
            int pointerId = event.getPointerId(i);
            float x = event.getX(i);
            float y = event.getY(i);
            float pressure = event.getPressure(i);

            NativeBridge.onHostTouchEvent(action, pointerId, x, y, pressure);
        }
        return true;
    }

    @Override
    public void surfaceCreated(SurfaceHolder holder) {
        NativeBridge.onSurfaceCreated(holder.getSurface());
    }

    @Override
    public void surfaceChanged(SurfaceHolder holder, int format, int width, int height) {
        NativeBridge.onSurfaceChanged(holder.getSurface(), width, height);
    }

    @Override
    public void surfaceDestroyed(SurfaceHolder holder) {
        NativeBridge.onSurfaceDestroyed();
    }
}
