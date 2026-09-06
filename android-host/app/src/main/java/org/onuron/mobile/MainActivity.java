package org.onuron.mobile;

import android.app.Activity;
import android.content.BroadcastReceiver;
import android.content.Context;
import android.content.Intent;
import android.content.IntentFilter;
import android.graphics.Canvas;
import android.graphics.Color;
import android.graphics.Paint;
import android.graphics.RectF;
import android.net.ConnectivityManager;
import android.net.NetworkInfo;
import android.net.Uri;
import android.os.BatteryManager;
import android.os.Bundle;
import android.os.Handler;
import android.os.Looper;
import android.os.Vibrator;
import android.util.Log;
import android.view.Gravity;
import android.view.KeyEvent;
import android.view.MotionEvent;
import android.view.View;
import android.view.ViewGroup;
import android.view.Window;
import android.view.WindowManager;
import android.view.inputmethod.EditorInfo;
import android.view.inputmethod.InputMethodManager;
import android.webkit.WebChromeClient;
import android.webkit.WebSettings;
import android.webkit.WebView;
import android.webkit.WebViewClient;
import android.widget.Button;
import android.widget.EditText;
import android.widget.FrameLayout;
import android.widget.LinearLayout;
import android.widget.TextView;

import java.io.BufferedReader;
import java.io.File;
import java.io.InputStreamReader;
import java.text.SimpleDateFormat;
import java.util.ArrayList;
import java.util.Date;
import java.util.List;
import java.util.Locale;

/**
 * MainActivity — Unified Onuron OS Mobile Host Runtime on Samsung Galaxy S25
 * Matches QEMU & Desktop Simulator with 1:1 UI, live browser, real calling, and interactive terminal.
 */
public class MainActivity extends Activity {

    private static final String TAG = "OnuronOS";
    private OnuronView onuronView;
    private FrameLayout rootLayout;
    private LinearLayout browserContainer;
    private WebView browserWebView;
    private EditText browserUrlBar;
    private LinearLayout terminalInputBar;
    private EditText terminalInputField;
    private final Handler mainHandler = new Handler(Looper.getMainLooper());

    private final BroadcastReceiver batteryReceiver = new BroadcastReceiver() {
        @Override
        public void onReceive(Context context, Intent intent) {
            int level = intent.getIntExtra(BatteryManager.EXTRA_LEVEL, -1);
            int scale = intent.getIntExtra(BatteryManager.EXTRA_SCALE, -1);
            int status = intent.getIntExtra(BatteryManager.EXTRA_STATUS, -1);

            if (level >= 0 && scale > 0 && onuronView != null) {
                onuronView.batteryLevel = (level * 100) / scale;
                onuronView.isCharging = (status == BatteryManager.BATTERY_STATUS_CHARGING
                        || status == BatteryManager.BATTERY_STATUS_FULL);
                onuronView.invalidate();
            }
        }
    };

    @Override
    protected void onCreate(Bundle savedInstanceState) {
        super.onCreate(savedInstanceState);

        // Crash guard: Catch any unexpected uncaught exceptions gracefully
        Thread.setDefaultUncaughtExceptionHandler((thread, throwable) -> {
            Log.e(TAG, "Uncaught exception in Onuron: " + throwable.getMessage(), throwable);
        });

        try {
            requestWindowFeature(Window.FEATURE_NO_TITLE);
            getWindow().setFlags(WindowManager.LayoutParams.FLAG_FULLSCREEN,
                    WindowManager.LayoutParams.FLAG_FULLSCREEN);
            getWindow().addFlags(WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON);

            View decorView = getWindow().getDecorView();
            decorView.setSystemUiVisibility(
                    View.SYSTEM_UI_FLAG_IMMERSIVE_STICKY
                            | View.SYSTEM_UI_FLAG_FULLSCREEN
                            | View.SYSTEM_UI_FLAG_HIDE_NAVIGATION
                            | View.SYSTEM_UI_FLAG_LAYOUT_FULLSCREEN
                            | View.SYSTEM_UI_FLAG_LAYOUT_HIDE_NAVIGATION
                            | View.SYSTEM_UI_FLAG_LAYOUT_STABLE
            );

            rootLayout = new FrameLayout(this);
            rootLayout.setBackgroundColor(0xFF0A0E17);

            // 1. Base Onuron Mobile View
            onuronView = new OnuronView(this, this);
            rootLayout.addView(onuronView, new FrameLayout.LayoutParams(
                    ViewGroup.LayoutParams.MATCH_PARENT, ViewGroup.LayoutParams.MATCH_PARENT));

            // 2. Embedded NilBrowser Container
            setupBrowserContainer();
            rootLayout.addView(browserContainer, new FrameLayout.LayoutParams(
                    ViewGroup.LayoutParams.MATCH_PARENT, ViewGroup.LayoutParams.MATCH_PARENT));

            // 3. Floating Terminal Input Bar
            setupTerminalInputBar();
            FrameLayout.LayoutParams termLp = new FrameLayout.LayoutParams(
                    ViewGroup.LayoutParams.MATCH_PARENT, ViewGroup.LayoutParams.WRAP_CONTENT);
            termLp.gravity = Gravity.BOTTOM;
            termLp.bottomMargin = (int) (56 * getResources().getDisplayMetrics().density);
            rootLayout.addView(terminalInputBar, termLp);

            setContentView(rootLayout);

            // Register real battery telemetry
            registerReceiver(batteryReceiver, new IntentFilter(Intent.ACTION_BATTERY_CHANGED));

            // Start hardware bridge service safely
            try {
                Intent serviceIntent = new Intent(this, OnuronBridgeService.class);
                startService(serviceIntent);
            } catch (Throwable t) {
                Log.w(TAG, "Bridge service startup skipped: " + t.getMessage());
            }

            // Periodic 1-second clock tick
            mainHandler.postDelayed(new Runnable() {
                @Override
                public void run() {
                    if (onuronView != null) {
                        onuronView.invalidate();
                    }
                    mainHandler.postDelayed(this, 1000);
                }
            }, 1000);

        } catch (Throwable t) {
            Log.e(TAG, "Fatal in onCreate", t);
        }
    }

    private void setupBrowserContainer() {
        browserContainer = new LinearLayout(this);
        browserContainer.setOrientation(LinearLayout.VERTICAL);
        browserContainer.setBackgroundColor(0xFF0A0E17);
        browserContainer.setVisibility(View.GONE);

        // Top Browser Header
        LinearLayout header = new LinearLayout(this);
        header.setOrientation(LinearLayout.HORIZONTAL);
        header.setBackgroundColor(0xFF141C2B);
        header.setGravity(Gravity.CENTER_VERTICAL);
        int pad = (int) (8 * getResources().getDisplayMetrics().density);
        header.setPadding(pad, pad + (int) (32 * getResources().getDisplayMetrics().density), pad, pad);

        // Close / Home Button
        Button closeBtn = new Button(this);
        closeBtn.setText("✕");
        closeBtn.setTextColor(0xFFFF5252);
        closeBtn.setBackgroundColor(0x00000000);
        closeBtn.setTextSize(18);
        closeBtn.setOnClickListener(v -> hideBrowser());
        header.addView(closeBtn, new LinearLayout.LayoutParams(
                (int) (44 * getResources().getDisplayMetrics().density), ViewGroup.LayoutParams.WRAP_CONTENT));

        // Back Button
        Button backBtn = new Button(this);
        backBtn.setText("◀");
        backBtn.setTextColor(0xFF00E5FF);
        backBtn.setBackgroundColor(0x00000000);
        backBtn.setTextSize(14);
        backBtn.setOnClickListener(v -> {
            if (browserWebView != null && browserWebView.canGoBack()) {
                browserWebView.goBack();
            }
        });
        header.addView(backBtn, new LinearLayout.LayoutParams(
                (int) (36 * getResources().getDisplayMetrics().density), ViewGroup.LayoutParams.WRAP_CONTENT));

        // URL Input Bar
        browserUrlBar = new EditText(this);
        browserUrlBar.setHint("ওয়েবসাইট বা সার্চ লিখুন (URL)...");
        browserUrlBar.setHintTextColor(0xFF475569);
        browserUrlBar.setTextColor(0xFFFFFFFF);
        browserUrlBar.setTextSize(13);
        browserUrlBar.setSingleLine(true);
        browserUrlBar.setImeOptions(EditorInfo.IME_ACTION_GO);
        browserUrlBar.setBackgroundColor(0xFF1E293B);
        browserUrlBar.setPadding(pad * 2, pad, pad * 2, pad);
        browserUrlBar.setOnEditorActionListener((v, actionId, event) -> {
            if (actionId == EditorInfo.IME_ACTION_GO || (event != null && event.getKeyCode() == KeyEvent.KEYCODE_ENTER)) {
                loadBrowserUrl(browserUrlBar.getText().toString());
                hideSoftKeyboard(browserUrlBar);
                return true;
            }
            return false;
        });

        LinearLayout.LayoutParams urlLp = new LinearLayout.LayoutParams(0, ViewGroup.LayoutParams.WRAP_CONTENT, 1.0f);
        urlLp.setMargins(pad, 0, pad, 0);
        header.addView(browserUrlBar, urlLp);

        // Go / Reload Button
        Button goBtn = new Button(this);
        goBtn.setText("➔");
        goBtn.setTextColor(0xFF00E676);
        goBtn.setBackgroundColor(0x00000000);
        goBtn.setTextSize(16);
        goBtn.setOnClickListener(v -> {
            loadBrowserUrl(browserUrlBar.getText().toString());
            hideSoftKeyboard(browserUrlBar);
        });
        header.addView(goBtn, new LinearLayout.LayoutParams(
                (int) (40 * getResources().getDisplayMetrics().density), ViewGroup.LayoutParams.WRAP_CONTENT));

        browserContainer.addView(header, new LinearLayout.LayoutParams(
                ViewGroup.LayoutParams.MATCH_PARENT, ViewGroup.LayoutParams.WRAP_CONTENT));

        // Web View
        browserWebView = new WebView(this);
        WebSettings ws = browserWebView.getSettings();
        ws.setJavaScriptEnabled(true);
        ws.setDomStorageEnabled(true);
        ws.setDatabaseEnabled(true);
        ws.setUseWideViewPort(true);
        ws.setLoadWithOverviewMode(true);
        ws.setSupportZoom(true);
        ws.setBuiltInZoomControls(true);
        ws.setDisplayZoomControls(false);

        browserWebView.setWebViewClient(new WebViewClient() {
            @Override
            public void onPageFinished(WebView view, String url) {
                if (browserUrlBar != null) {
                    browserUrlBar.setText(url);
                }
            }
        });
        browserWebView.setWebChromeClient(new WebChromeClient());

        browserContainer.addView(browserWebView, new LinearLayout.LayoutParams(
                ViewGroup.LayoutParams.MATCH_PARENT, ViewGroup.LayoutParams.MATCH_PARENT));
    }

    private void setupTerminalInputBar() {
        terminalInputBar = new LinearLayout(this);
        terminalInputBar.setOrientation(LinearLayout.HORIZONTAL);
        terminalInputBar.setBackgroundColor(0xFF141C2B);
        terminalInputBar.setGravity(Gravity.CENTER_VERTICAL);
        terminalInputBar.setVisibility(View.GONE);
        int pad = (int) (8 * getResources().getDisplayMetrics().density);
        terminalInputBar.setPadding(pad, pad, pad, pad);

        TextView prompt = new TextView(this);
        prompt.setText("nilos$ ");
        prompt.setTextColor(0xFF00E676);
        prompt.setTextSize(14);
        terminalInputBar.addView(prompt);

        terminalInputField = new EditText(this);
        terminalInputField.setHint("কমান্ড লিখুন (e.g. ls -la, pwd, uname)...");
        terminalInputField.setHintTextColor(0xFF475569);
        terminalInputField.setTextColor(0xFFFFFFFF);
        terminalInputField.setTextSize(13);
        terminalInputField.setSingleLine(true);
        terminalInputField.setBackgroundColor(0xFF1E293B);
        terminalInputField.setPadding(pad, pad, pad, pad);
        terminalInputField.setImeOptions(EditorInfo.IME_ACTION_DONE);
        terminalInputField.setOnEditorActionListener((v, actionId, event) -> {
            if (actionId == EditorInfo.IME_ACTION_DONE || (event != null && event.getKeyCode() == KeyEvent.KEYCODE_ENTER)) {
                String cmd = terminalInputField.getText().toString().trim();
                if (!cmd.isEmpty() && onuronView != null) {
                    onuronView.runCommand(cmd);
                    terminalInputField.setText("");
                }
                return true;
            }
            return false;
        });

        LinearLayout.LayoutParams inputLp = new LinearLayout.LayoutParams(0, ViewGroup.LayoutParams.WRAP_CONTENT, 1.0f);
        inputLp.setMargins(pad, 0, pad, 0);
        terminalInputBar.addView(terminalInputField, inputLp);

        Button runBtn = new Button(this);
        runBtn.setText("RUN");
        runBtn.setTextColor(0xFF0A0E17);
        runBtn.setBackgroundColor(0xFF00E676);
        runBtn.setTextSize(12);
        runBtn.setOnClickListener(v -> {
            String cmd = terminalInputField.getText().toString().trim();
            if (!cmd.isEmpty() && onuronView != null) {
                onuronView.runCommand(cmd);
                terminalInputField.setText("");
            }
        });
        terminalInputBar.addView(runBtn, new LinearLayout.LayoutParams(
                (int) (64 * getResources().getDisplayMetrics().density), (int) (38 * getResources().getDisplayMetrics().density)));
    }

    public void showBrowser(String url) {
        if (browserContainer != null) {
            browserContainer.setVisibility(View.VISIBLE);
            loadBrowserUrl(url != null && !url.isEmpty() ? url : "https://duckduckgo.com");
        }
    }

    public void hideBrowser() {
        if (browserContainer != null) {
            browserContainer.setVisibility(View.GONE);
        }
    }

    private void loadBrowserUrl(String url) {
        if (url == null || url.trim().isEmpty()) return;
        String target = url.trim();
        if (!target.startsWith("http://") && !target.startsWith("https://")) {
            if (target.contains(".") && !target.contains(" ")) {
                target = "https://" + target;
            } else {
                target = "https://duckduckgo.com/?q=" + Uri.encode(target);
            }
        }
        if (browserWebView != null) {
            browserWebView.loadUrl(target);
        }
    }

    public void setTerminalInputVisible(boolean visible) {
        if (terminalInputBar != null) {
            terminalInputBar.setVisibility(visible ? View.VISIBLE : View.GONE);
            if (visible && terminalInputField != null) {
                terminalInputField.requestFocus();
                showSoftKeyboard(terminalInputField);
            } else if (!visible && terminalInputField != null) {
                hideSoftKeyboard(terminalInputField);
            }
        }
    }

    public void triggerRealCall(String number) {
        if (number == null || number.isEmpty()) return;
        try {
            Intent callIntent = new Intent(Intent.ACTION_DIAL);
            callIntent.setData(Uri.parse("tel:" + number));
            startActivity(callIntent);
        } catch (Exception e) {
            Log.e(TAG, "Failed to start phone dialer intent", e);
        }
    }

    private void showSoftKeyboard(View view) {
        InputMethodManager imm = (InputMethodManager) getSystemService(Context.INPUT_METHOD_SERVICE);
        if (imm != null) {
            imm.showSoftInput(view, InputMethodManager.SHOW_IMPLICIT);
        }
    }

    private void hideSoftKeyboard(View view) {
        InputMethodManager imm = (InputMethodManager) getSystemService(Context.INPUT_METHOD_SERVICE);
        if (imm != null) {
            imm.hideSoftInputFromWindow(view.getWindowToken(), 0);
        }
    }

    @Override
    public void onBackPressed() {
        if (browserContainer != null && browserContainer.getVisibility() == View.VISIBLE) {
            if (browserWebView != null && browserWebView.canGoBack()) {
                browserWebView.goBack();
            } else {
                hideBrowser();
            }
            return;
        }
        if (onuronView != null && onuronView.screen != OnuronView.Screen.HOME) {
            onuronView.screen = OnuronView.Screen.HOME;
            setTerminalInputVisible(false);
            onuronView.invalidate();
            return;
        }
        super.onBackPressed();
    }

    @Override
    protected void onDestroy() {
        super.onDestroy();
        try {
            unregisterReceiver(batteryReceiver);
        } catch (Exception ignored) {
        }
    }

    // ─── Interactive Onuron OS Mobile View ─────────────────────────────────────────

    public static class OnuronView extends View {

        enum Screen {
            HOME, PHONE, MESSAGES, FILES, SETTINGS, TERMINAL, ABOUT
        }

        private static class TouchArea {
            RectF bounds;
            String id;

            TouchArea(RectF bounds, String id) {
                this.bounds = bounds;
                this.id = id;
            }
        }

        // Color Palette
        private static final int COLOR_BG = 0xFF0A0E17;
        private static final int COLOR_SURFACE = 0xFF141C2B;
        private static final int COLOR_SURFACE_ALT = 0xFF1E293B;
        private static final int COLOR_BORDER = 0xFF2D3748;
        private static final int COLOR_CYAN = 0xFF00E5FF;
        private static final int COLOR_BLUE = 0xFF2979FF;
        private static final int COLOR_GREEN = 0xFF00E676;
        private static final int COLOR_AMBER = 0xFFFFB300;
        private static final int COLOR_PURPLE = 0xFF7C4DFF;
        private static final int COLOR_RED = 0xFFFF5252;
        private static final int COLOR_TEXT_HIGH = 0xFFFFFFFF;
        private static final int COLOR_TEXT_MED = 0xFF94A3B8;
        private static final int COLOR_TEXT_DIM = 0xFF475569;

        private final Paint paint = new Paint(Paint.ANTI_ALIAS_FLAG);
        private final List<TouchArea> touchAreas = new ArrayList<>();
        private final Vibrator vibrator;
        private final MainActivity activity;

        // Dynamic State
        public Screen screen = Screen.HOME;
        public boolean wifiEnabled = true;
        public int wifiSignalLevel = 3;
        public boolean cellularEnabled = true;
        public int cellularSignalLevel = 4;
        public int batteryLevel = 95;
        public boolean isCharging = false;
        public boolean darkMode = true;

        // Phone App State
        public String dialNumber = "";

        // Files App State
        public String currentPath;

        // Terminal App State
        public List<String> termLines = new ArrayList<>();
        public String termCwd;

        public OnuronView(Context context, MainActivity activity) {
            super(context);
            this.activity = activity;
            this.vibrator = (Vibrator) context.getSystemService(Context.VIBRATOR_SERVICE);

            // Set safe user directories where 'ls' and file operations ALWAYS succeed without permission denied
            File appFiles = context.getFilesDir();
            this.termCwd = appFiles.getAbsolutePath();
            this.currentPath = appFiles.getAbsolutePath();

            // Create some default Onuron user files if missing
            try {
                new File(appFiles, "system.conf").createNewFile();
                new File(appFiles, "onuron_apps").mkdir();
                new File(appFiles, "downloads").mkdir();
            } catch (Exception ignored) {
            }

            // Initialize terminal banner
            termLines.add("==================================================");
            termLines.add("   Onuron OS Terminal CLI (Samsung Galaxy S25)   ");
            termLines.add("   Hardware: Snapdragon 8 Elite • Android Host   ");
            termLines.add("==================================================");
            termLines.add("Working Dir: " + termCwd);
            termLines.add("Tap quick chips or type below to run commands.");

            checkSystemNetwork();
        }

        private void checkSystemNetwork() {
            try {
                ConnectivityManager cm = (ConnectivityManager) getContext().getSystemService(Context.CONNECTIVITY_SERVICE);
                if (cm != null) {
                    NetworkInfo active = cm.getActiveNetworkInfo();
                    if (active != null && active.isConnected()) {
                        if (active.getType() == ConnectivityManager.TYPE_WIFI) {
                            wifiEnabled = true;
                            wifiSignalLevel = 3;
                        } else if (active.getType() == ConnectivityManager.TYPE_MOBILE) {
                            cellularEnabled = true;
                            cellularSignalLevel = 4;
                        }
                    }
                }
            } catch (Exception ignored) {
            }
        }

        private void vibrate() {
            try {
                if (vibrator != null && vibrator.hasVibrator()) {
                    vibrator.vibrate(25);
                }
            } catch (Exception ignored) {
            }
        }

        private String toBengaliDigits(String s) {
            StringBuilder sb = new StringBuilder();
            for (char c : s.toCharArray()) {
                switch (c) {
                    case '0': sb.append('০'); break;
                    case '1': sb.append('১'); break;
                    case '2': sb.append('২'); break;
                    case '3': sb.append('৩'); break;
                    case '4': sb.append('৪'); break;
                    case '5': sb.append('৫'); break;
                    case '6': sb.append('৬'); break;
                    case '7': sb.append('৭'); break;
                    case '8': sb.append('৮'); break;
                    case '9': sb.append('৯'); break;
                    default: sb.append(c);
                }
            }
            return sb.toString();
        }

        @Override
        protected void onDraw(Canvas canvas) {
            super.onDraw(canvas);
            touchAreas.clear();

            int w = getWidth();
            int h = getHeight();
            if (w <= 0 || h <= 0) return;

            // 1. Background
            paint.setStyle(Paint.Style.FILL);
            paint.setColor(COLOR_BG);
            canvas.drawRect(0, 0, w, h, paint);

            // 2. Content Screen
            switch (screen) {
                case HOME:
                    renderHomeScreen(canvas, w, h);
                    break;
                case PHONE:
                    renderPhoneScreen(canvas, w, h);
                    break;
                case SETTINGS:
                    renderSettingsScreen(canvas, w, h);
                    break;
                case MESSAGES:
                    renderMessagesScreen(canvas, w, h);
                    break;
                case FILES:
                    renderFilesScreen(canvas, w, h);
                    break;
                case TERMINAL:
                    renderTerminalScreen(canvas, w, h);
                    break;
                case ABOUT:
                    renderAboutScreen(canvas, w, h);
                    break;
            }

            // 3. Top Dynamic Status Bar (Universal on top of all screens)
            renderStatusBar(canvas, w);

            // 4. Bottom Navigation Bar
            renderBottomNav(canvas, w, h);
        }

        // ─── Status Bar ─────────────────────────────────────────────────────────────

        private void renderStatusBar(Canvas canvas, int w) {
            int barHeight = dp(42);

            paint.setStyle(Paint.Style.FILL);
            paint.setColor(0xEE080C14);
            canvas.drawRect(0, 0, w, barHeight, paint);

            paint.setColor(COLOR_BORDER);
            canvas.drawRect(0, barHeight - 1, w, barHeight, paint);

            // Left: Real Time in Bengali
            SimpleDateFormat sdf = new SimpleDateFormat("HH:mm", Locale.getDefault());
            String timeStr = toBengaliDigits(sdf.format(new Date()));
            paint.setColor(COLOR_TEXT_HIGH);
            paint.setTextSize(dp(14));
            paint.setTextAlign(Paint.Align.LEFT);
            paint.setFakeBoldText(true);
            canvas.drawText(timeStr, dp(16), dp(26), paint);

            // Center: Dynamic Island
            float pillW = dp(84);
            float pillH = dp(24);
            float pillX = (w - pillW) / 2f;
            float pillY = dp(9);
            paint.setColor(Color.BLACK);
            canvas.drawRoundRect(pillX, pillY, pillX + pillW, pillY + pillH, pillH / 2f, pillH / 2f, paint);

            paint.setColor(0xFF1E293B);
            canvas.drawCircle(pillX + dp(18), pillY + pillH / 2f, dp(4), paint);
            paint.setColor(0xFF0F172A);
            canvas.drawCircle(pillX + pillW - dp(24), pillY + pillH / 2f, dp(5), paint);

            touchAreas.add(new TouchArea(new RectF(pillX, 0, pillX + pillW, barHeight), "toggle_island"));

            // Right side: Dynamic Indicators
            float rightX = w - dp(16);

            // 1. Dynamic Battery
            float batW = dp(24);
            float batH = dp(13);
            float batX = rightX - batW;
            float batY = dp(14);

            paint.setStyle(Paint.Style.STROKE);
            paint.setColor(COLOR_TEXT_MED);
            paint.setStrokeWidth(dp(1.5f));
            canvas.drawRoundRect(batX, batY, batX + batW, batY + batH, dp(3), dp(3), paint);

            paint.setStyle(Paint.Style.FILL);
            canvas.drawRoundRect(batX + batW, batY + dp(3.5f), batX + batW + dp(2.5f), batY + batH - dp(3.5f), dp(1), dp(1), paint);

            float fillPct = Math.max(0.05f, Math.min(1f, batteryLevel / 100f));
            float innerW = (batW - dp(4)) * fillPct;
            int batColor = batteryLevel > 40 ? COLOR_GREEN : (batteryLevel > 20 ? COLOR_AMBER : COLOR_RED);
            paint.setColor(batColor);
            canvas.drawRoundRect(batX + dp(2), batY + dp(2), batX + dp(2) + innerW, batY + batH - dp(2), dp(2), dp(2), paint);

            if (isCharging) {
                paint.setColor(COLOR_AMBER);
                paint.setTextSize(dp(10));
                paint.setTextAlign(Paint.Align.CENTER);
                canvas.drawText("⚡", batX + batW / 2f, batY + batH - dp(2), paint);
            }

            touchAreas.add(new TouchArea(new RectF(batX - dp(4), 0, w, barHeight), "toggle_battery"));
            rightX = batX - dp(10);

            // 2. Dynamic Cellular Signal Tower (4 vertical bars)
            float towerW = dp(18);
            float towerBaseY = dp(27);
            float[] barHeights = {dp(3.5f), dp(6.5f), dp(9.5f), dp(13f)};

            paint.setStyle(Paint.Style.FILL);
            float towerX = rightX - towerW;
            if (cellularEnabled) {
                for (int i = 0; i < 4; i++) {
                    float bx = towerX + i * dp(4.5f);
                    float by = towerBaseY - barHeights[i];
                    paint.setColor(i < cellularSignalLevel ? COLOR_TEXT_HIGH : 0xFF2D3748);
                    canvas.drawRoundRect(bx, by, bx + dp(2.5f), towerBaseY, dp(1), dp(1), paint);
                }
            } else {
                paint.setColor(0xFF2D3748);
                for (int i = 0; i < 4; i++) {
                    float bx = towerX + i * dp(4.5f);
                    float by = towerBaseY - barHeights[i];
                    canvas.drawRoundRect(bx, by, bx + dp(2.5f), towerBaseY, dp(1), dp(1), paint);
                }
                paint.setColor(COLOR_RED);
                paint.setTextSize(dp(9));
                paint.setTextAlign(Paint.Align.CENTER);
                canvas.drawText("✕", towerX + towerW / 2f, towerBaseY - dp(2), paint);
            }

            rightX = towerX - dp(6);

            // Cellular Generation Badge
            String netBadge;
            int netColor;
            if (cellularEnabled) {
                switch (cellularSignalLevel) {
                    case 4:
                    case 3:
                        netBadge = "৫G";
                        netColor = COLOR_CYAN;
                        break;
                    case 2:
                        netBadge = "৪G";
                        netColor = COLOR_BLUE;
                        break;
                    case 1:
                        netBadge = "৩G";
                        netColor = COLOR_AMBER;
                        break;
                    default:
                        netBadge = "E";
                        netColor = COLOR_RED;
                }
            } else {
                netBadge = "অফ";
                netColor = COLOR_TEXT_DIM;
            }

            paint.setColor(netColor);
            paint.setTextSize(dp(11));
            paint.setTextAlign(Paint.Align.RIGHT);
            paint.setFakeBoldText(true);
            canvas.drawText(netBadge, rightX, dp(25), paint);

            touchAreas.add(new TouchArea(new RectF(rightX - dp(24), 0, towerX + towerW, barHeight), "cycle_signal"));
            rightX -= dp(28);

            // 3. Dynamic Wi-Fi (3 concentric arcs)
            float wifiW = dp(16);
            float wifiX = rightX - wifiW;

            if (wifiEnabled) {
                paint.setStyle(Paint.Style.STROKE);
                paint.setStrokeWidth(dp(1.8f));
                paint.setColor(wifiSignalLevel >= 3 ? COLOR_CYAN : 0xFF2D3748);
                canvas.drawArc(new RectF(wifiX, dp(13), wifiX + wifiW, dp(29)), 220, 100, false, paint);

                paint.setColor(wifiSignalLevel >= 2 ? COLOR_CYAN : 0xFF2D3748);
                canvas.drawArc(new RectF(wifiX + dp(3), dp(17), wifiX + wifiW - dp(3), dp(29)), 220, 100, false, paint);

                paint.setStyle(Paint.Style.FILL);
                paint.setColor(wifiSignalLevel >= 1 ? COLOR_CYAN : 0xFF2D3748);
                canvas.drawCircle(wifiX + wifiW / 2f, dp(26), dp(1.8f), paint);
            } else {
                paint.setStyle(Paint.Style.STROKE);
                paint.setStrokeWidth(dp(1.5f));
                paint.setColor(0xFF2D3748);
                canvas.drawArc(new RectF(wifiX, dp(13), wifiX + wifiW, dp(29)), 220, 100, false, paint);
                paint.setStyle(Paint.Style.FILL);
                paint.setColor(COLOR_RED);
                paint.setTextSize(dp(9));
                paint.setTextAlign(Paint.Align.CENTER);
                canvas.drawText("✕", wifiX + wifiW / 2f, dp(25), paint);
            }

            touchAreas.add(new TouchArea(new RectF(wifiX - dp(6), 0, rightX + dp(4), barHeight), "toggle_wifi"));
        }

        // ─── Home Screen ─────────────────────────────────────────────────────────────

        private void renderHomeScreen(Canvas canvas, int w, int h) {
            float startY = dp(60);

            // Hero Digital Clock
            SimpleDateFormat sdfHour = new SimpleDateFormat("HH:mm", Locale.getDefault());
            String clockStr = toBengaliDigits(sdfHour.format(new Date()));
            paint.setColor(COLOR_TEXT_HIGH);
            paint.setTextSize(dp(56));
            paint.setTextAlign(Paint.Align.CENTER);
            paint.setFakeBoldText(true);
            canvas.drawText(clockStr, w / 2f, startY + dp(52), paint);

            // Date in Bengali
            SimpleDateFormat sdfDate = new SimpleDateFormat("EEEE, d MMMM yyyy", new Locale("bn", "BD"));
            String dateStr = sdfDate.format(new Date());
            paint.setColor(COLOR_CYAN);
            paint.setTextSize(dp(14.5f));
            paint.setFakeBoldText(false);
            canvas.drawText(dateStr, w / 2f, startY + dp(82), paint);

            // System Chip Card
            float chipY = startY + dp(104);
            paint.setStyle(Paint.Style.FILL);
            paint.setColor(COLOR_SURFACE);
            RectF chipRect = new RectF(dp(18), chipY, w - dp(18), chipY + dp(44));
            canvas.drawRoundRect(chipRect, dp(12), dp(12), paint);
            paint.setStyle(Paint.Style.STROKE);
            paint.setColor(COLOR_BORDER);
            paint.setStrokeWidth(dp(1));
            canvas.drawRoundRect(chipRect, dp(12), dp(12), paint);

            paint.setStyle(Paint.Style.FILL);
            paint.setColor(COLOR_TEXT_MED);
            paint.setTextSize(dp(12));
            paint.setTextAlign(Paint.Align.CENTER);
            canvas.drawText("⚡ Snapdragon 8 Elite • 120Hz Dynamic AMOLED • Onuron 1.0", w / 2f, chipY + dp(27), paint);

            // App Grid (2 rows x 4 columns)
            float gridY = chipY + dp(66);
            int cols = 4;
            float iconSize = dp(60);
            float colWidth = (w - dp(32)) / (float) cols;

            AppItem[] apps = {
                    new AppItem("ফোন", "app_phone", "📞", COLOR_GREEN),
                    new AppItem("ব্রাউজার", "app_browser", "🌐", COLOR_CYAN),
                    new AppItem("বার্তা", "app_messages", "💬", COLOR_AMBER),
                    new AppItem("ফাইল", "app_files", "📁", COLOR_BLUE),
                    new AppItem("সেটিংস", "app_settings", "⚙️", COLOR_PURPLE),
                    new AppItem("টার্মিনাল", "app_terminal", "💻", COLOR_CYAN),
                    new AppItem("সফটবাস", "app_softbus", "⚡", COLOR_GREEN),
                    new AppItem("অনুরণ", "app_about", "ℹ️", COLOR_TEXT_MED),
            };

            for (int i = 0; i < apps.length; i++) {
                int r = i / cols;
                int c = i % cols;
                float cx = dp(16) + c * colWidth + colWidth / 2f;
                float cy = gridY + r * dp(102);

                RectF iconRect = new RectF(cx - iconSize / 2f, cy, cx + iconSize / 2f, cy + iconSize);
                paint.setStyle(Paint.Style.FILL);
                paint.setColor(COLOR_SURFACE);
                canvas.drawRoundRect(iconRect, dp(18), dp(18), paint);

                paint.setStyle(Paint.Style.STROKE);
                paint.setColor(COLOR_BORDER);
                paint.setStrokeWidth(dp(1));
                canvas.drawRoundRect(iconRect, dp(18), dp(18), paint);

                paint.setStyle(Paint.Style.FILL);
                paint.setColor(apps[i].accent);
                canvas.drawCircle(cx, cy + dp(14), dp(3.5f), paint);

                paint.setTextSize(dp(24));
                paint.setTextAlign(Paint.Align.CENTER);
                canvas.drawText(apps[i].emoji, cx, cy + dp(43), paint);

                paint.setColor(COLOR_TEXT_HIGH);
                paint.setTextSize(dp(12.5f));
                paint.setFakeBoldText(false);
                canvas.drawText(apps[i].title, cx, cy + iconSize + dp(20), paint);

                touchAreas.add(new TouchArea(iconRect, apps[i].id));
            }
        }

        // ─── Phone Screen (Full Proportional S25 Sizing) ───────────────────────────

        private void renderPhoneScreen(Canvas canvas, int w, int h) {
            float y = dp(52);
            paint.setColor(COLOR_GREEN);
            paint.setTextSize(dp(22));
            paint.setFakeBoldText(true);
            paint.setTextAlign(Paint.Align.LEFT);
            canvas.drawText("ফোন ডায়ালার (Phone)", dp(20), y + dp(22), paint);

            // Number display card
            y += dp(36);
            RectF numCard = new RectF(dp(20), y, w - dp(20), y + dp(64));
            paint.setStyle(Paint.Style.FILL);
            paint.setColor(COLOR_SURFACE);
            canvas.drawRoundRect(numCard, dp(14), dp(14), paint);
            paint.setStyle(Paint.Style.STROKE);
            paint.setColor(COLOR_BORDER);
            canvas.drawRoundRect(numCard, dp(14), dp(14), paint);

            paint.setStyle(Paint.Style.FILL);
            paint.setColor(COLOR_TEXT_HIGH);
            paint.setTextSize(dp(24));
            paint.setTextAlign(Paint.Align.CENTER);
            String display = dialNumber.isEmpty() ? "নম্বর লিখুন..." : toBengaliDigits(dialNumber);
            canvas.drawText(display, w / 2f, y + dp(41), paint);

            // Dynamically scale keypad to fill tall S25 screen without empty void
            float bottomNavH = dp(54);
            float availableH = (h - bottomNavH) - (y + dp(74)) - dp(84);
            float btnH = Math.min(dp(72), availableH / 4f - dp(12));
            float btnW = (w - dp(64)) / 3f;
            float padStartY = y + dp(80);

            String[][] pad = {
                    {"1", "2", "3"},
                    {"4", "5", "6"},
                    {"7", "8", "9"},
                    {"*", "0", "#"}
            };

            for (int r = 0; r < 4; r++) {
                for (int c = 0; c < 3; c++) {
                    float bx = dp(20) + c * (btnW + dp(12));
                    float by = padStartY + r * (btnH + dp(12));
                    RectF btnRect = new RectF(bx, by, bx + btnW, by + btnH);

                    paint.setStyle(Paint.Style.FILL);
                    paint.setColor(COLOR_SURFACE_ALT);
                    canvas.drawRoundRect(btnRect, dp(16), dp(16), paint);
                    paint.setStyle(Paint.Style.STROKE);
                    paint.setColor(COLOR_BORDER);
                    canvas.drawRoundRect(btnRect, dp(16), dp(16), paint);

                    paint.setStyle(Paint.Style.FILL);
                    paint.setColor(COLOR_TEXT_HIGH);
                    paint.setTextSize(dp(24));
                    paint.setTextAlign(Paint.Align.CENTER);
                    canvas.drawText(toBengaliDigits(pad[r][c]), bx + btnW / 2f, by + btnH / 2f + dp(9), paint);

                    touchAreas.add(new TouchArea(btnRect, "key_" + pad[r][c]));
                }
            }

            // Call & Delete buttons (stretched and prominent)
            float actY = padStartY + 4 * (btnH + dp(12)) + dp(8);
            float callBtnW = w - dp(104);
            RectF callRect = new RectF(dp(20), actY, dp(20) + callBtnW, actY + dp(58));

            paint.setStyle(Paint.Style.FILL);
            paint.setColor(COLOR_GREEN);
            canvas.drawRoundRect(callRect, dp(16), dp(16), paint);

            paint.setColor(COLOR_BG);
            paint.setTextSize(dp(18));
            paint.setFakeBoldText(true);
            paint.setTextAlign(Paint.Align.CENTER);
            canvas.drawText("📞 কল করুন (Call)", callRect.centerX(), actY + dp(36), paint);
            touchAreas.add(new TouchArea(callRect, "act_call"));

            RectF delRect = new RectF(dp(28) + callBtnW, actY, w - dp(20), actY + dp(58));
            paint.setColor(COLOR_SURFACE_ALT);
            canvas.drawRoundRect(delRect, dp(16), dp(16), paint);

            paint.setColor(COLOR_RED);
            paint.setTextSize(dp(22));
            canvas.drawText("⌫", delRect.centerX(), actY + dp(37), paint);
            touchAreas.add(new TouchArea(delRect, "act_del"));
        }

        // ─── Terminal Screen ────────────────────────────────────────────────────────

        private void renderTerminalScreen(Canvas canvas, int w, int h) {
            float y = dp(52);
            paint.setColor(COLOR_CYAN);
            paint.setTextSize(dp(20));
            paint.setFakeBoldText(true);
            paint.setTextAlign(Paint.Align.LEFT);
            canvas.drawText("টার্মিনাল (Terminal CLI)", dp(20), y + dp(22), paint);

            y += dp(36);

            // Quick Chips
            String[] chips = {"ls -la", "pwd", "uname -a", "date", "df -h", "clear"};
            float chipW = (w - dp(52)) / 3f;
            for (int i = 0; i < chips.length; i++) {
                int r = i / 3;
                int c = i % 3;
                float cx = dp(20) + c * (chipW + dp(6));
                float cy = y + r * dp(34);
                RectF chipRect = new RectF(cx, cy, cx + chipW, cy + dp(28));

                paint.setStyle(Paint.Style.FILL);
                paint.setColor(COLOR_SURFACE);
                canvas.drawRoundRect(chipRect, dp(8), dp(8), paint);

                paint.setColor(COLOR_CYAN);
                paint.setTextSize(dp(12));
                paint.setTextAlign(Paint.Align.CENTER);
                canvas.drawText(chips[i], cx + chipW / 2f, cy + dp(18), paint);

                touchAreas.add(new TouchArea(chipRect, "chip_" + chips[i]));
            }

            y += dp(78);

            // Terminal Screen Output Box
            float boxBottom = h - dp(116);
            RectF termBox = new RectF(dp(20), y, w - dp(20), boxBottom);
            paint.setStyle(Paint.Style.FILL);
            paint.setColor(0xFF040609);
            canvas.drawRoundRect(termBox, dp(12), dp(12), paint);

            paint.setStyle(Paint.Style.STROKE);
            paint.setColor(COLOR_BORDER);
            canvas.drawRoundRect(termBox, dp(12), dp(12), paint);

            // Render terminal output lines from bottom up
            paint.setStyle(Paint.Style.FILL);
            paint.setTextSize(dp(11.5f));
            paint.setTextAlign(Paint.Align.LEFT);

            float lineY = y + dp(22);
            int maxLines = (int) ((termBox.height() - dp(24)) / dp(18));
            int startIdx = Math.max(0, termLines.size() - maxLines);

            for (int i = startIdx; i < termLines.size(); i++) {
                String l = termLines.get(i);
                if (l.startsWith("$") || l.startsWith("nilos")) {
                    paint.setColor(COLOR_GREEN);
                } else if (l.startsWith("=")) {
                    paint.setColor(COLOR_CYAN);
                } else if (l.startsWith("!")) {
                    paint.setColor(COLOR_RED);
                } else {
                    paint.setColor(COLOR_TEXT_MED);
                }
                canvas.drawText(l, dp(30), lineY, paint);
                lineY += dp(18);
            }

            // Register tap on terminal box to open soft keyboard
            touchAreas.add(new TouchArea(termBox, "open_keyboard"));
        }

        public void runCommand(String rawCmd) {
            String cmd = rawCmd.trim();
            termLines.add("nilos:" + termCwd + "$ " + cmd);

            if ("clear".equals(cmd)) {
                termLines.clear();
                invalidate();
                return;
            }

            try {
                // Execute in safe app files directory where ls, mkdir, echo always have full permissions!
                Process p = Runtime.getRuntime().exec(new String[]{"sh", "-c", cmd}, null, new File(termCwd));
                BufferedReader reader = new BufferedReader(new InputStreamReader(p.getInputStream()));
                String line;
                while ((line = reader.readLine()) != null) {
                    termLines.add(line);
                }
                BufferedReader errReader = new BufferedReader(new InputStreamReader(p.getErrorStream()));
                while ((line = errReader.readLine()) != null) {
                    termLines.add("! " + line);
                }
                p.waitFor();
            } catch (Exception e) {
                termLines.add("! Error executing command: " + e.getMessage());
            }

            while (termLines.size() > 70) {
                termLines.remove(0);
            }
            invalidate();
        }

        // ─── Settings Screen ────────────────────────────────────────────────────────

        private void renderSettingsScreen(Canvas canvas, int w, int h) {
            float y = dp(52);

            paint.setColor(COLOR_PURPLE);
            paint.setTextSize(dp(22));
            paint.setFakeBoldText(true);
            paint.setTextAlign(Paint.Align.LEFT);
            canvas.drawText("সেটিংস (Settings)", dp(20), y + dp(22), paint);

            y += dp(40);

            SettingToggle[] toggles = {
                    new SettingToggle("ওয়াইফাই নেটওয়ার্ক (Wi-Fi)", wifiEnabled ? "[ চালু - ৩/৩ ]" : "[ বন্ধ ]", wifiEnabled ? COLOR_GREEN : COLOR_TEXT_DIM, "toggle_wifi"),
                    new SettingToggle("সেলুলার ৫G ডেটা (Cellular)", cellularEnabled ? "[ চালু ]" : "[ বন্ধ ]", cellularEnabled ? COLOR_GREEN : COLOR_TEXT_DIM, "toggle_cellular"),
                    new SettingToggle("সিগন্যাল টাওয়ার ক্ষমতা (Tower)", getSignalTowerLabel(), COLOR_CYAN, "cycle_signal"),
                    new SettingToggle("ব্যাটারি পাওয়ার স্টেট", isCharging ? "[ চার্জ হচ্ছে ⚡ ]" : "[ সাধারণ " + batteryLevel + "% ]", isCharging ? COLOR_AMBER : COLOR_GREEN, "toggle_battery"),
                    new SettingToggle("ডার্ক মোড থিম (Theme)", darkMode ? "[ চালু ]" : "[ বন্ধ ]", darkMode ? COLOR_GREEN : COLOR_TEXT_DIM, "toggle_dark"),
            };

            for (SettingToggle t : toggles) {
                RectF card = new RectF(dp(20), y, w - dp(20), y + dp(52));
                paint.setStyle(Paint.Style.FILL);
                paint.setColor(COLOR_SURFACE);
                canvas.drawRoundRect(card, dp(12), dp(12), paint);

                paint.setStyle(Paint.Style.STROKE);
                paint.setColor(COLOR_BORDER);
                paint.setStrokeWidth(dp(1));
                canvas.drawRoundRect(card, dp(12), dp(12), paint);

                paint.setStyle(Paint.Style.FILL);
                paint.setColor(COLOR_TEXT_HIGH);
                paint.setTextSize(dp(14));
                paint.setFakeBoldText(false);
                paint.setTextAlign(Paint.Align.LEFT);
                canvas.drawText(t.title, dp(34), y + dp(32), paint);

                paint.setColor(t.color);
                paint.setFakeBoldText(true);
                paint.setTextAlign(Paint.Align.RIGHT);
                canvas.drawText(t.status, w - dp(34), y + dp(32), paint);

                touchAreas.add(new TouchArea(card, t.id));
                y += dp(60);
            }

            // Hardware Info Card
            RectF infoCard = new RectF(dp(20), y, w - dp(20), y + dp(130));
            paint.setStyle(Paint.Style.FILL);
            paint.setColor(COLOR_SURFACE_ALT);
            canvas.drawRoundRect(infoCard, dp(12), dp(12), paint);

            paint.setColor(COLOR_CYAN);
            paint.setTextSize(dp(14));
            paint.setFakeBoldText(true);
            paint.setTextAlign(Paint.Align.LEFT);
            canvas.drawText("ডিভাইস ও সিস্টেম তথ্য", dp(34), y + dp(28), paint);

            paint.setColor(COLOR_TEXT_MED);
            paint.setTextSize(dp(12));
            paint.setFakeBoldText(false);
            canvas.drawText("• ডিভাইস: Samsung Galaxy S25 (SM-S931B)", dp(34), y + dp(54), paint);
            canvas.drawText("• প্রসেসর: Qualcomm Snapdragon 8 Elite (৩ ন্যানোমিটার)", dp(34), y + dp(78), paint);
            canvas.drawText("• ওএস কার্নেল: Onuron NilHAL 1.0 (Linux 6.6 LTS)", dp(34), y + dp(102), paint);
        }

        private String getSignalTowerLabel() {
            if (!cellularEnabled) return "[ অফ ✕ ]";
            switch (cellularSignalLevel) {
                case 4: return "[ ৪/৪ বার - ৫G ]";
                case 3: return "[ ৩/৪ বার - ৫G ]";
                case 2: return "[ ২/৪ বার - ৪G ]";
                case 1: return "[ ১/৪ বার - ৩G ]";
                default: return "[ ০/৪ বার - E ]";
            }
        }

        // ─── Messages Screen ────────────────────────────────────────────────────────

        private void renderMessagesScreen(Canvas canvas, int w, int h) {
            float y = dp(52);
            paint.setColor(COLOR_AMBER);
            paint.setTextSize(dp(22));
            paint.setFakeBoldText(true);
            paint.setTextAlign(Paint.Align.LEFT);
            canvas.drawText("বার্তা (Messages SMS)", dp(20), y + dp(22), paint);

            y += dp(40);
            String[][] threads = {
                    {"Onuron System", "fscrypt v2 মেমরি এনক্রিপশন সক্রিয় রয়েছে।", "১২:৪৫"},
                    {"SoftBus Mesh", "NilPad-Pro-X1 পেয়ারিংয়ের জন্য প্রস্তুত।", "১২:৪২"},
                    {"NilPkg Store", "সব সিস্টেম সার্ভিস আপ-টু-ডেট রয়েছে।", "১১:৩০"}
            };

            for (String[] th : threads) {
                RectF card = new RectF(dp(20), y, w - dp(20), y + dp(64));
                paint.setStyle(Paint.Style.FILL);
                paint.setColor(COLOR_SURFACE);
                canvas.drawRoundRect(card, dp(12), dp(12), paint);

                paint.setColor(COLOR_CYAN);
                paint.setTextSize(dp(14));
                paint.setFakeBoldText(true);
                paint.setTextAlign(Paint.Align.LEFT);
                canvas.drawText(th[0], dp(34), y + dp(26), paint);

                paint.setColor(COLOR_TEXT_MED);
                paint.setTextSize(dp(11));
                paint.setTextAlign(Paint.Align.RIGHT);
                canvas.drawText(th[2], w - dp(34), y + dp(26), paint);

                paint.setColor(COLOR_TEXT_HIGH);
                paint.setTextSize(dp(12));
                paint.setFakeBoldText(false);
                paint.setTextAlign(Paint.Align.LEFT);
                canvas.drawText(th[1], dp(34), y + dp(48), paint);

                y += dp(74);
            }
        }

        // ─── Files Screen ───────────────────────────────────────────────────────────

        private void renderFilesScreen(Canvas canvas, int w, int h) {
            float y = dp(52);
            paint.setColor(COLOR_BLUE);
            paint.setTextSize(dp(22));
            paint.setFakeBoldText(true);
            paint.setTextAlign(Paint.Align.LEFT);
            canvas.drawText("ফাইল ম্যানেজার (Files)", dp(20), y + dp(22), paint);

            y += dp(40);
            paint.setColor(COLOR_TEXT_MED);
            paint.setTextSize(dp(12));
            canvas.drawText("পাথ: " + currentPath, dp(20), y + dp(14), paint);

            y += dp(26);
            File dir = new File(currentPath);
            File[] files = dir.listFiles();
            if (files != null && files.length > 0) {
                int count = 0;
                for (File f : files) {
                    if (count++ > 8) break;
                    RectF card = new RectF(dp(20), y, w - dp(20), y + dp(42));
                    paint.setStyle(Paint.Style.FILL);
                    paint.setColor(COLOR_SURFACE);
                    canvas.drawRoundRect(card, dp(8), dp(8), paint);

                    String icon = f.isDirectory() ? "📁 [DIR] " : "📄 [FILE] ";
                    paint.setColor(f.isDirectory() ? COLOR_CYAN : COLOR_TEXT_HIGH);
                    paint.setTextSize(dp(13));
                    paint.setTextAlign(Paint.Align.LEFT);
                    canvas.drawText(icon + f.getName(), dp(32), y + dp(26), paint);

                    y += dp(48);
                }
            } else {
                paint.setColor(COLOR_TEXT_DIM);
                canvas.drawText("ডিরেক্টরি খালি অথবা সিস্টেম পারমিশন সক্রিয় নয়।", dp(20), y + dp(20), paint);
            }
        }

        // ─── About Screen ───────────────────────────────────────────────────────────

        private void renderAboutScreen(Canvas canvas, int w, int h) {
            float y = dp(52);
            paint.setColor(COLOR_CYAN);
            paint.setTextSize(dp(22));
            paint.setFakeBoldText(true);
            paint.setTextAlign(Paint.Align.LEFT);
            canvas.drawText("Onuron OS সম্পর্কে", dp(20), y + dp(22), paint);

            y += dp(46);
            RectF card = new RectF(dp(20), y, w - dp(20), y + dp(220));
            paint.setStyle(Paint.Style.FILL);
            paint.setColor(COLOR_SURFACE);
            canvas.drawRoundRect(card, dp(14), dp(14), paint);

            paint.setStyle(Paint.Style.STROKE);
            paint.setColor(COLOR_BORDER);
            canvas.drawRoundRect(card, dp(14), dp(14), paint);

            paint.setStyle(Paint.Style.FILL);
            paint.setTextSize(dp(13));
            paint.setFakeBoldText(false);
            paint.setTextAlign(Paint.Align.LEFT);

            float ty = y + dp(32);
            String[] lines = {
                    "📘 Onuron OS (অনুরণ ওএস) — v1.0.0-alpha",
                    "• মেমরি-নিরাপদ Rust ইউজারস্পেস",
                    "• Alap রিঅ্যাক্টিভ মোবাইল UI ফ্রেমওয়ার্ক",
                    "• Samsung Galaxy S25 Snapdragon 8 Elite টেস্ট ল্যাব",
                    "• ১২০Hz ডায়নামিক AMOLED ২X অপ্টিমাইজড",
                    "• শূন্য ট্র্যাকিং ও ১০০% উন্মুক্ত সোর্স (GPLv3)",
                    "• SoftBus সমন্বিত ডিভাইস মেশ আর্কিটেকচার"
            };

            for (String l : lines) {
                paint.setColor(l.startsWith("📘") ? COLOR_CYAN : COLOR_TEXT_MED);
                canvas.drawText(l, dp(34), ty, paint);
                ty += dp(24);
            }
        }

        // ─── Bottom Navigation Bar ──────────────────────────────────────────────────

        private void renderBottomNav(Canvas canvas, int w, int h) {
            int navH = dp(52);
            float navY = h - navH;

            paint.setStyle(Paint.Style.FILL);
            paint.setColor(0xFF0C121E);
            canvas.drawRect(0, navY, w, h, paint);

            paint.setColor(COLOR_BORDER);
            canvas.drawRect(0, navY, w, navY + 1, paint);

            float btnW = (w - dp(32)) / 3f;
            float btnH = dp(38);
            float btnY = navY + dp(7);

            // Back button
            RectF backRect = new RectF(dp(8), btnY, dp(8) + btnW, btnY + btnH);
            paint.setColor(COLOR_SURFACE);
            canvas.drawRoundRect(backRect, dp(10), dp(10), paint);
            paint.setColor(COLOR_TEXT_MED);
            paint.setTextSize(dp(13));
            paint.setTextAlign(Paint.Align.CENTER);
            canvas.drawText("◀ ব্যাক (Back)", backRect.centerX(), btnY + dp(24), paint);
            touchAreas.add(new TouchArea(backRect, "nav_back"));

            // Home button
            RectF homeRect = new RectF(dp(16) + btnW, btnY, dp(16) + 2 * btnW, btnY + btnH);
            paint.setColor(screen == Screen.HOME ? 0xFF0C2E55 : COLOR_SURFACE);
            canvas.drawRoundRect(homeRect, dp(10), dp(10), paint);
            paint.setColor(COLOR_CYAN);
            canvas.drawText("⌂ হোম (Home)", homeRect.centerX(), btnY + dp(24), paint);
            touchAreas.add(new TouchArea(homeRect, "nav_home"));

            // Lock button
            RectF lockRect = new RectF(dp(24) + 2 * btnW, btnY, w - dp(8), btnY + btnH);
            paint.setColor(COLOR_SURFACE);
            canvas.drawRoundRect(lockRect, dp(10), dp(10), paint);
            paint.setColor(COLOR_AMBER);
            canvas.drawText("⏻ মেনু (Menu)", lockRect.centerX(), btnY + dp(24), paint);
            touchAreas.add(new TouchArea(lockRect, "nav_menu"));
        }

        // ─── Touch Event Dispatcher ─────────────────────────────────────────────────

        @Override
        public boolean onTouchEvent(MotionEvent event) {
            if (event.getAction() == MotionEvent.ACTION_DOWN) {
                float x = event.getX();
                float y = event.getY();

                for (TouchArea area : touchAreas) {
                    if (area.bounds.contains(x, y)) {
                        handleAction(area.id);
                        vibrate();
                        invalidate();
                        return true;
                    }
                }
            }
            return true;
        }

        private void handleAction(String id) {
            switch (id) {
                case "nav_home":
                case "nav_back":
                    screen = Screen.HOME;
                    activity.setTerminalInputVisible(false);
                    activity.hideBrowser();
                    break;
                case "nav_menu":
                    screen = (screen == Screen.SETTINGS ? Screen.HOME : Screen.SETTINGS);
                    activity.setTerminalInputVisible(false);
                    activity.hideBrowser();
                    break;
                case "app_phone":
                    screen = Screen.PHONE;
                    activity.setTerminalInputVisible(false);
                    activity.hideBrowser();
                    break;
                case "app_browser":
                    activity.showBrowser("https://duckduckgo.com");
                    break;
                case "app_settings":
                    screen = Screen.SETTINGS;
                    activity.setTerminalInputVisible(false);
                    activity.hideBrowser();
                    break;
                case "app_messages":
                    screen = Screen.MESSAGES;
                    activity.setTerminalInputVisible(false);
                    activity.hideBrowser();
                    break;
                case "app_files":
                    screen = Screen.FILES;
                    activity.setTerminalInputVisible(false);
                    activity.hideBrowser();
                    break;
                case "app_terminal":
                    screen = Screen.TERMINAL;
                    activity.setTerminalInputVisible(true);
                    activity.hideBrowser();
                    break;
                case "app_about":
                case "app_softbus":
                    screen = Screen.ABOUT;
                    activity.setTerminalInputVisible(false);
                    activity.hideBrowser();
                    break;
                case "open_keyboard":
                    activity.setTerminalInputVisible(true);
                    break;
                case "toggle_wifi":
                    wifiEnabled = !wifiEnabled;
                    break;
                case "toggle_cellular":
                    cellularEnabled = !cellularEnabled;
                    break;
                case "cycle_signal":
                    if (cellularEnabled) {
                        cellularSignalLevel = (cellularSignalLevel == 0) ? 4 : (cellularSignalLevel - 1);
                    } else {
                        cellularEnabled = true;
                        cellularSignalLevel = 4;
                    }
                    break;
                case "toggle_battery":
                    isCharging = !isCharging;
                    if (isCharging) {
                        batteryLevel = 100;
                    } else {
                        batteryLevel = 78;
                    }
                    break;
                case "toggle_dark":
                    darkMode = !darkMode;
                    break;
                case "act_call":
                    if (!dialNumber.isEmpty()) {
                        activity.triggerRealCall(dialNumber);
                    }
                    break;
                case "act_del":
                    if (!dialNumber.isEmpty()) {
                        dialNumber = dialNumber.substring(0, dialNumber.length() - 1);
                    }
                    break;
                default:
                    if (id.startsWith("key_")) {
                        dialNumber += id.substring(4);
                    } else if (id.startsWith("chip_")) {
                        runCommand(id.substring(5));
                    }
            }
        }

        private float dp(float val) {
            return val * getResources().getDisplayMetrics().density;
        }
        private int dp(int val) {
            return (int) (val * getResources().getDisplayMetrics().density);
        }

        private static class AppItem {
            String title;
            String id;
            String emoji;
            int accent;

            AppItem(String title, String id, String emoji, int accent) {
                this.title = title;
                this.id = id;
                this.emoji = emoji;
                this.accent = accent;
            }
        }

        private static class SettingToggle {
            String title;
            String status;
            int color;
            String id;

            SettingToggle(String title, String status, int color, String id) {
                this.title = title;
                this.status = status;
                this.color = color;
                this.id = id;
            }
        }
    }
}
