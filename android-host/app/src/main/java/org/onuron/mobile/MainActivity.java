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
import android.widget.Toast;

import java.io.BufferedReader;
import java.io.File;
import java.io.InputStreamReader;
import java.text.SimpleDateFormat;
import java.util.ArrayList;
import java.util.Date;
import java.util.List;
import java.util.Locale;

/**
 * MainActivity — Unified Onuron OS Mobile Host Runtime
 * Implements all 35 screens matching the official Onuron OS Design System
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
    private LinearLayout messageInputBar;
    private EditText messageInputField;
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
            rootLayout.setBackgroundColor(0xFF1E1236);

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

            // 4. Floating Messages Input Bar
            setupMessageInputBar();
            FrameLayout.LayoutParams msgLp = new FrameLayout.LayoutParams(
                    ViewGroup.LayoutParams.MATCH_PARENT, ViewGroup.LayoutParams.WRAP_CONTENT);
            msgLp.gravity = Gravity.BOTTOM;
            msgLp.bottomMargin = (int) (56 * getResources().getDisplayMetrics().density);
            rootLayout.addView(messageInputBar, msgLp);

            setContentView(rootLayout);

            // Register battery telemetry
            registerReceiver(batteryReceiver, new IntentFilter(Intent.ACTION_BATTERY_CHANGED));

            // Start hardware bridge service
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
                        onuronView.onTick();
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

        Button backBtn = new Button(this);
        backBtn.setText("✕");
        backBtn.setTextColor(0xFFFFFFFF);
        backBtn.setBackgroundColor(0x00000000);
        backBtn.setTextSize(18);
        backBtn.setOnClickListener(v -> hideBrowser());
        header.addView(backBtn, new LinearLayout.LayoutParams(
                (int) (44 * getResources().getDisplayMetrics().density), (int) (40 * getResources().getDisplayMetrics().density)));

        browserUrlBar = new EditText(this);
        browserUrlBar.setHint("ওয়েবসাইট বা সার্চ লিখুন...");
        browserUrlBar.setHintTextColor(0xFF475569);
        browserUrlBar.setTextColor(0xFFFFFFFF);
        browserUrlBar.setTextSize(13);
        browserUrlBar.setSingleLine(true);
        browserUrlBar.setBackgroundColor(0xFF1E293B);
        browserUrlBar.setPadding(pad, pad, pad, pad);
        browserUrlBar.setImeOptions(EditorInfo.IME_ACTION_GO);
        browserUrlBar.setOnEditorActionListener((v, actionId, event) -> {
            if (actionId == EditorInfo.IME_ACTION_GO || (event != null && event.getKeyCode() == KeyEvent.KEYCODE_ENTER)) {
                String input = browserUrlBar.getText().toString().trim();
                if (!input.isEmpty()) {
                    loadBrowserUrl(input);
                }
                return true;
            }
            return false;
        });

        LinearLayout.LayoutParams urlLp = new LinearLayout.LayoutParams(0, ViewGroup.LayoutParams.WRAP_CONTENT, 1.0f);
        urlLp.setMargins(pad, 0, pad, 0);
        header.addView(browserUrlBar, urlLp);

        Button goBtn = new Button(this);
        goBtn.setText("যাও");
        goBtn.setTextColor(0xFF0A0E17);
        goBtn.setBackgroundColor(0xFF7C3AED);
        goBtn.setTextSize(12);
        goBtn.setOnClickListener(v -> {
            String input = browserUrlBar.getText().toString().trim();
            if (!input.isEmpty()) {
                loadBrowserUrl(input);
            }
        });
        header.addView(goBtn, new LinearLayout.LayoutParams(
                (int) (52 * getResources().getDisplayMetrics().density), (int) (38 * getResources().getDisplayMetrics().density)));

        browserContainer.addView(header, new LinearLayout.LayoutParams(
                ViewGroup.LayoutParams.MATCH_PARENT, ViewGroup.LayoutParams.WRAP_CONTENT));

        // WebView
        try {
            browserWebView = new WebView(this);
            WebSettings ws = browserWebView.getSettings();
            ws.setJavaScriptEnabled(true);
            ws.setDomStorageEnabled(true);
            ws.setLoadWithOverviewMode(true);
            ws.setUseWideViewPort(true);
            browserWebView.setWebViewClient(new WebViewClient() {
                @Override
                public void onPageFinished(WebView view, String url) {
                    super.onPageFinished(view, url);
                    if (browserUrlBar != null) {
                        browserUrlBar.setText(url);
                    }
                }
            });
            browserWebView.setWebChromeClient(new WebChromeClient());
            browserContainer.addView(browserWebView, new LinearLayout.LayoutParams(
                    ViewGroup.LayoutParams.MATCH_PARENT, ViewGroup.LayoutParams.MATCH_PARENT));
        } catch (Throwable t) {
            Log.e(TAG, "WebView init error", t);
        }
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
        prompt.setText("nilshell> ");
        prompt.setTextColor(0xFF00E676);
        prompt.setTextSize(13);
        terminalInputBar.addView(prompt);

        terminalInputField = new EditText(this);
        terminalInputField.setHint("কমান্ড লিখুন...");
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

    private void setupMessageInputBar() {
        messageInputBar = new LinearLayout(this);
        messageInputBar.setOrientation(LinearLayout.HORIZONTAL);
        messageInputBar.setBackgroundColor(0xFF141C2B);
        messageInputBar.setGravity(Gravity.CENTER_VERTICAL);
        messageInputBar.setVisibility(View.GONE);
        int pad = (int) (8 * getResources().getDisplayMetrics().density);
        messageInputBar.setPadding(pad, pad, pad, pad);

        messageInputField = new EditText(this);
        messageInputField.setHint("বার্তা লিখুন...");
        messageInputField.setHintTextColor(0xFF475569);
        messageInputField.setTextColor(0xFFFFFFFF);
        messageInputField.setTextSize(13);
        messageInputField.setSingleLine(true);
        messageInputField.setBackgroundColor(0xFF1E293B);
        messageInputField.setPadding(pad, pad, pad, pad);
        messageInputField.setImeOptions(EditorInfo.IME_ACTION_SEND);
        messageInputField.setOnEditorActionListener((v, actionId, event) -> {
            if (actionId == EditorInfo.IME_ACTION_SEND || (event != null && event.getKeyCode() == KeyEvent.KEYCODE_ENTER)) {
                String msg = messageInputField.getText().toString().trim();
                if (!msg.isEmpty() && onuronView != null) {
                    onuronView.sendMessage(msg);
                    messageInputField.setText("");
                }
                return true;
            }
            return false;
        });

        LinearLayout.LayoutParams inputLp = new LinearLayout.LayoutParams(0, ViewGroup.LayoutParams.WRAP_CONTENT, 1.0f);
        inputLp.setMargins(pad, 0, pad, 0);
        messageInputBar.addView(messageInputField, inputLp);

        Button sendBtn = new Button(this);
        sendBtn.setText("পাঠান");
        sendBtn.setTextColor(0xFFFFFFFF);
        sendBtn.setBackgroundColor(0xFF7C3AED);
        sendBtn.setTextSize(13);
        sendBtn.setOnClickListener(v -> {
            String msg = messageInputField.getText().toString().trim();
            if (!msg.isEmpty() && onuronView != null) {
                onuronView.sendMessage(msg);
                messageInputField.setText("");
            }
        });
        messageInputBar.addView(sendBtn, new LinearLayout.LayoutParams(
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
            if (browserWebView != null) {
                browserWebView.stopLoading();
            }
        }
    }

    private void loadBrowserUrl(String input) {
        if (browserWebView == null) return;
        String url = input.trim();
        if (!url.startsWith("http://") && !url.startsWith("https://")) {
            if (url.contains(".") && !url.contains(" ")) {
                url = "https://" + url;
            } else {
                url = "https://duckduckgo.com/?q=" + Uri.encode(url);
            }
        }
        if (browserUrlBar != null) {
            browserUrlBar.setText(url);
        }
        browserWebView.loadUrl(url);
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

    public void setMessageInputVisible(boolean visible) {
        if (messageInputBar != null) {
            messageInputBar.setVisibility(visible ? View.VISIBLE : View.GONE);
            if (visible && messageInputField != null) {
                messageInputField.requestFocus();
                showSoftKeyboard(messageInputField);
            } else if (!visible && messageInputField != null) {
                hideSoftKeyboard(messageInputField);
            }
        }
    }

    public void triggerRealCall(String number) {
        if (number == null || number.isEmpty()) return;
        try {
            if (onuronView != null) {
                SimpleDateFormat sdf = new SimpleDateFormat("HH:mm", Locale.getDefault());
                onuronView.callLogs.add(0, new String[]{number, sdf.format(new Date()), "আউটগোয়িং"});
            }
            Intent callIntent = new Intent(Intent.ACTION_DIAL);
            callIntent.setData(Uri.parse("tel:" + number));
            startActivity(callIntent);
        } catch (Exception e) {
            Log.e(TAG, "Failed to start phone dialer intent", e);
        }
    }

    public void triggerCamera() {
        try {
            Intent cameraIntent = new Intent(android.provider.MediaStore.ACTION_IMAGE_CAPTURE);
            if (cameraIntent.resolveActivity(getPackageManager()) != null) {
                startActivity(cameraIntent);
            } else {
                Toast.makeText(this, "ক্যামেরা অ্যাপ্লিকেশন সক্রিয়", Toast.LENGTH_SHORT).show();
            }
        } catch (Exception e) {
            Log.e(TAG, "Camera launch error", e);
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
            onuronView.handleBack();
            return;
        }
        super.onBackPressed();
    }

    @Override
    protected void onPause() {
        super.onPause();
        NativeBridge.pushHostEventJson("{\"type\":\"HostPause\"}");
    }

    @Override
    protected void onResume() {
        super.onResume();
        NativeBridge.pushHostEventJson("{\"type\":\"HostResume\"}");
    }

    @Override
    protected void onDestroy() {
        super.onDestroy();
        try {
            unregisterReceiver(batteryReceiver);
        } catch (Exception ignored) {
        }
        NativeBridge.onSurfaceDestroyed();
    }


    // ─── Interactive Onuron OS Mobile View ─────────────────────────────────────────

    public static class OnuronView extends View {

        public enum Screen {
            LOCK,
            HOME,
            CONTROL_CENTER,
            RECENTS,
            POWER_MENU,
            PHONE,
            IN_CALL,
            MESSAGES,
            FILES,
            SETTINGS,
            SETTINGS_WIFI,
            SETTINGS_BLUETOOTH,
            SETTINGS_DISPLAY,
            SETTINGS_SOUND,
            SETTINGS_SECURITY,
            SETTINGS_BATTERY,
            SETTINGS_STORAGE,
            SETTINGS_LANGUAGE,
            SETTINGS_ACCESSIBILITY,
            SETTINGS_VPN,
            SETTINGS_BACKUP,
            SETTINGS_UPDATE,
            TERMINAL,
            ABOUT,
            NOTES,
            MUSIC,
            CALCULATOR,
            CAMERA,
            SOFTBUS,
            NILPKG,
            ANDROID_CONTAINER,
            OOBE_1,
            OOBE_2,
            OOBE_3
        }

        private static class TouchArea {
            RectF bounds;
            String id;

            TouchArea(RectF bounds, String id) {
                this.bounds = bounds;
                this.id = id;
            }
        }

        private final Paint paint = new Paint(Paint.ANTI_ALIAS_FLAG);
        private final List<TouchArea> touchAreas = new ArrayList<>();
        private final Vibrator vibrator;
        private final MainActivity activity;

        // Dynamic State
        public Screen screen = Screen.HOME;
        public Screen prevScreen = Screen.HOME;
        public boolean wifiEnabled = true;
        public int wifiSignalLevel = 3;
        public boolean cellularEnabled = true;
        public int cellularSignalLevel = 4;
        public int batteryLevel = 95;
        public boolean isCharging = false;
        public boolean darkMode = true;
        public boolean vibratorEnabled = true;
        public int soundVolume = 75;
        public int brightnessLevel = 85;
        public boolean torchEnabled = false;
        public boolean btEnabled = false;
        public boolean airplaneMode = false;
        public boolean locationEnabled = true;
        public boolean batterySaver = false;

        // In-Call state
        public int inCallSec = 24;
        public boolean inCallMuted = false;
        public boolean inCallSpeaker = false;

        // Phone App State
        public int phoneTab = 0; // 0: Dialpad, 1: Recents, 2: Contacts
        public String dialNumber = "০১৭৮৯-১২৩৪৫৬";
        public List<String[]> contacts = new ArrayList<>();
        public List<String[]> callLogs = new ArrayList<>();

        // Messages App State
        public List<String[]> messageThreads = new ArrayList<>();
        public List<String[]> activeChatMessages = new ArrayList<>();
        public String activeRecipient = "অনিমেষ";
        public boolean inActiveChat = false;

        // Files App State
        public int filesFilter = 0; // 0: All, 1: Docs, 2: Pics, 3: Audio, 4: Downloads
        public String currentPath = "/data/storage";
        public String selectedFileInfo = null;

        // Terminal App State
        public List<String> termLines = new ArrayList<>();
        public String termCwd;

        // Calculator App State
        public String calcDisplay = "৩,৭৫০";
        public String calcFormula = "১২৫০ × ৩ =";
        public String calcRawInput = "3750";

        // Notes App State
        public int notesCategory = 0; // 0: সব, 1: কাজ, 2: ব্যক্তিগত, 3: জরুরি
        public List<String[]> notesList = new ArrayList<>(); // [Title, Content, Tag, Date]

        // Music App State
        public boolean musicPlaying = false;
        public int musicCurrentTrack = 0;
        public int musicProgressSec = 105;
        public List<String[]> musicTracks = new ArrayList<>();

        // Camera App State
        public int cameraZoom = 1; // 0: 0.5x, 1: 1x, 2: 2x
        public int cameraMode = 0; // 0: Photo, 1: Video
        public boolean cameraFlash = false;
        public int shutterFlashAnim = 0;

        // Recents tasks
        public List<String> recentsList = new ArrayList<>();

        // Settings Toggles
        public boolean biometricEnabled = true;
        public boolean vpnConnected = false;
        public boolean clipSync = true;

        public OnuronView(Context context, MainActivity activity) {
            super(context);
            this.activity = activity;
            this.vibrator = (Vibrator) context.getSystemService(Context.VIBRATOR_SERVICE);

            File appFiles = context.getFilesDir();
            this.termCwd = appFiles.getAbsolutePath();

            // Contacts
            contacts.add(new String[]{"অনিমেষ", "+8801789-123456"});
            contacts.add(new String[]{"সুমিত্রা", "+8801812-345678"});
            contacts.add(new String[]{"মা", "+8801711-223344"});
            contacts.add(new String[]{"বাবা", "+8801722-334455"});
            contacts.add(new String[]{"রাজদীপ", "+8801933-445566"});

            // Call Logs
            callLogs.add(new String[]{"অনিমেষ", "১০:২৪", "ইনকামিং"});
            callLogs.add(new String[]{"সুমিত্রা", "গতকাল", "আউটগোয়িং"});
            callLogs.add(new String[]{"মা", "রবিবার", "মিসড"});

            // Message Threads
            messageThreads.add(new String[]{"অনিমেষ", "কালকের সভার সময় কখন?", "১০:২৪"});
            messageThreads.add(new String[]{"সুমিত্রা", "ছবিগুলো পাঠিয়ে দিয়েছি, দেখে নিও।", "গতকাল"});
            messageThreads.add(new String[]{"মা", "বাড়ি পৌঁছালে কল করিস।", "রবিবার"});
            messageThreads.add(new String[]{"রাজদীপ", "প্রজেক্ট ফাইল আপডেট করা হয়েছে।", "শুক্রবার"});
            messageThreads.add(new String[]{"তনুশ্রী", "শুভ জন্মদিন! 🎉", "১২ অক্টো"});

            activeChatMessages.add(new String[]{"অনিমেষ", "নমস্কার! কেমন আছো?", "১০:২০"});
            activeChatMessages.add(new String[]{"me", "ভালো। তোমার খবর কি?", "১০:২২"});
            activeChatMessages.add(new String[]{"অনিমেষ", "কালকের সভার সময় কখন?", "১০:২৪"});

            // Notes
            notesList.add(new String[]{"বাজার তালিকা", "সবজি, ফল, দুধ, ডিম, মসলা...", "জরুরি", "আজ"});
            notesList.add(new String[]{"মিটিং-এর পয়েন্ট", "১. সেশনের এজেন্ডা\n২. আর্কিটেকচার রিভিউ...", "কাজ", "গতকাল"});
            notesList.add(new String[]{"বই পড়ার তালিকা", "গীতাঞ্জলি, চাঁদের পাহাড়, আরণ্যক...", "ব্যক্তিগত", "১০ অক্টো"});

            // Music Tracks
            musicTracks.add(new String[]{"Cyber Bangla 2040", "NilLang Sonic Lab", "03:50"});
            musicTracks.add(new String[]{"Onuron Ambient Waves", "Alap Audio Engine", "04:12"});
            musicTracks.add(new String[]{"Snapdragon 8 Elite Groove", "Qualcomm DSP Team", "02:45"});
            musicTracks.add(new String[]{"Alap Reactive Symphony", "Onuron Native Sound", "05:01"});

            // Terminal Lines
            termLines.add("Onuron OS Terminal CLI v1.2.0 [Snapdragon 8 Elite]");
            termLines.add("Kernel: Linux LTS 6.6.21-onuron • PID 1: nilinit");
            termLines.add("Commands: help, ls, pwd, uname, clear, cat");

            // Recents initial list
            recentsList.add("ফোন");
            recentsList.add("বার্তা");
            recentsList.add("ফাইল");
            recentsList.add("নোটস");

            checkSystemNetwork();
        }

        public void onTick() {
            if (screen == Screen.IN_CALL) {
                inCallSec++;
            }
            if (musicPlaying) {
                musicProgressSec = (musicProgressSec + 1) % 230;
            }
            if (shutterFlashAnim > 0) {
                shutterFlashAnim--;
            }
        }

        public void handleBack() {
            activity.setTerminalInputVisible(false);
            activity.setMessageInputVisible(false);
            activity.hideBrowser();

            if (screen == Screen.IN_CALL) {
                screen = Screen.PHONE;
            } else if (screen == Screen.CONTROL_CENTER || screen == Screen.RECENTS || screen == Screen.POWER_MENU) {
                screen = prevScreen;
            } else if (isSettingsSubScreen(screen)) {
                screen = Screen.SETTINGS;
            } else if (inActiveChat) {
                inActiveChat = false;
            } else {
                screen = Screen.HOME;
            }
            invalidate();
        }

        private boolean isSettingsSubScreen(Screen s) {
            return s == Screen.SETTINGS_WIFI || s == Screen.SETTINGS_BLUETOOTH
                    || s == Screen.SETTINGS_DISPLAY || s == Screen.SETTINGS_SOUND
                    || s == Screen.SETTINGS_SECURITY || s == Screen.SETTINGS_BATTERY
                    || s == Screen.SETTINGS_STORAGE || s == Screen.SETTINGS_LANGUAGE
                    || s == Screen.SETTINGS_ACCESSIBILITY || s == Screen.SETTINGS_VPN
                    || s == Screen.SETTINGS_BACKUP || s == Screen.SETTINGS_UPDATE;
        }

        // Colors
        public int getBgColor() { return darkMode ? 0xFF1E1236 : 0xFFF5F0FF; }
        public int getSurfaceColor() { return darkMode ? 0xFF322748 : 0xFFFDFCFF; }
        public int getSurfaceAltColor() { return darkMode ? 0xFF3B2E56 : 0xFFEDE5FF; }
        public int getBorderColor() { return darkMode ? 0xFF4A3B65 : 0xFFE2D9F3; }
        public int getTextHighColor() { return darkMode ? 0xFFFFFFFF : 0xFF1E1236; }
        public int getTextMedColor() { return darkMode ? 0xFFB0A0B0 : 0xFF6B6272; }
        public int getTextDimColor() { return darkMode ? 0xFF7D728A : 0xFF9E97A6; }

        public static final int COLOR_CYAN = 0xFF7C3AED;
        public static final int COLOR_BLUE = 0xFF6366F1;
        public static final int COLOR_GREEN = 0xFF10B981;
        public static final int COLOR_AMBER = 0xFF8B5CF6;
        public static final int COLOR_PURPLE = 0xFF7C3AED;
        public static final int COLOR_RED = 0xFFEF4444;

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
                if (vibratorEnabled && vibrator != null && vibrator.hasVibrator()) {
                    vibrator.vibrate(25);
                }
            } catch (Exception ignored) {
            }
        }

        public String toBengaliDigits(String s) {
            if (s == null) return "";
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

        public String fromBengaliDigits(String s) {
            if (s == null) return "";
            StringBuilder sb = new StringBuilder();
            for (char c : s.toCharArray()) {
                switch (c) {
                    case '০': sb.append('0'); break;
                    case '১': sb.append('1'); break;
                    case '২': sb.append('2'); break;
                    case '৩': sb.append('3'); break;
                    case '৪': sb.append('4'); break;
                    case '৫': sb.append('5'); break;
                    case '৬': sb.append('6'); break;
                    case '৭': sb.append('7'); break;
                    case '৮': sb.append('8'); break;
                    case '৯': sb.append('9'); break;
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

            // 1. Canvas Background
            paint.setStyle(Paint.Style.FILL);
            paint.setColor(getBgColor());
            canvas.drawRect(0, 0, w, h, paint);

            // 2. Active Screen
            switch (screen) {
                case LOCK:
                    renderLockScreen(canvas, w, h);
                    break;
                case HOME:
                    renderHomeScreen(canvas, w, h);
                    break;
                case CONTROL_CENTER:
                    renderControlCenter(canvas, w, h);
                    break;
                case RECENTS:
                    renderRecentsScreen(canvas, w, h);
                    break;
                case POWER_MENU:
                    renderPowerMenuScreen(canvas, w, h);
                    break;
                case PHONE:
                    renderPhoneScreen(canvas, w, h);
                    break;
                case IN_CALL:
                    renderInCallScreen(canvas, w, h);
                    break;
                case MESSAGES:
                    renderMessagesScreen(canvas, w, h);
                    break;
                case FILES:
                    renderFilesScreen(canvas, w, h);
                    break;
                case CAMERA:
                    renderCameraScreen(canvas, w, h);
                    break;
                case CALCULATOR:
                    renderCalculatorScreen(canvas, w, h);
                    break;
                case NOTES:
                    renderNotesScreen(canvas, w, h);
                    break;
                case MUSIC:
                    renderMusicScreen(canvas, w, h);
                    break;
                case TERMINAL:
                    renderTerminalScreen(canvas, w, h);
                    break;
                case SOFTBUS:
                    renderSoftBusScreen(canvas, w, h);
                    break;
                case NILPKG:
                    renderNilPkgScreen(canvas, w, h);
                    break;
                case ANDROID_CONTAINER:
                    renderAndroidContainerScreen(canvas, w, h);
                    break;
                case OOBE_1:
                case OOBE_2:
                case OOBE_3:
                    renderOobeScreen(canvas, w, h);
                    break;
                case SETTINGS:
                    renderSettingsScreen(canvas, w, h);
                    break;
                case SETTINGS_WIFI:
                    renderSettingsWifi(canvas, w, h);
                    break;
                case SETTINGS_BLUETOOTH:
                    renderSettingsBluetooth(canvas, w, h);
                    break;
                case SETTINGS_DISPLAY:
                    renderSettingsDisplay(canvas, w, h);
                    break;
                case SETTINGS_SOUND:
                    renderSettingsSound(canvas, w, h);
                    break;
                case SETTINGS_SECURITY:
                    renderSettingsSecurity(canvas, w, h);
                    break;
                case SETTINGS_BATTERY:
                    renderSettingsBattery(canvas, w, h);
                    break;
                case SETTINGS_STORAGE:
                    renderSettingsStorage(canvas, w, h);
                    break;
                case SETTINGS_LANGUAGE:
                    renderSettingsLanguage(canvas, w, h);
                    break;
                case SETTINGS_ACCESSIBILITY:
                    renderSettingsAccessibility(canvas, w, h);
                    break;
                case SETTINGS_VPN:
                    renderSettingsVpn(canvas, w, h);
                    break;
                case SETTINGS_BACKUP:
                    renderSettingsBackup(canvas, w, h);
                    break;
                case SETTINGS_UPDATE:
                    renderSettingsUpdate(canvas, w, h);
                    break;
                case ABOUT:
                    renderAboutScreen(canvas, w, h);
                    break;
            }

            // Top Status Bar (overlay, unless in camera or lock)
            if (screen != Screen.CAMERA && screen != Screen.LOCK && screen != Screen.POWER_MENU) {
                renderStatusBar(canvas, w);
            }

            // Bottom Navigation Bar (overlay, unless in control center, lock or power menu)
            if (screen != Screen.CONTROL_CENTER && screen != Screen.LOCK && screen != Screen.POWER_MENU) {
                renderBottomNav(canvas, w, h);
            }

            // Camera shutter flash animation overlay
            if (shutterFlashAnim > 0) {
                paint.setColor(0x88FFFFFF);
                canvas.drawRect(0, 0, w, h, paint);
            }
        }

        // ─── Top Universal Status Bar ───────────────────────────────────────────────

        private void renderStatusBar(Canvas canvas, int w) {
            int barHeight = dp(42);

            paint.setStyle(Paint.Style.FILL);
            paint.setColor(darkMode ? 0xEE1E1236 : 0xEEF5F0FF);
            canvas.drawRect(0, 0, w, barHeight, paint);

            paint.setColor(getBorderColor());
            canvas.drawRect(0, barHeight - 1, w, barHeight, paint);

            // Left: Real Time in Bengali
            SimpleDateFormat sdf = new SimpleDateFormat("HH:mm", Locale.getDefault());
            String timeStr = toBengaliDigits(sdf.format(new Date()));
            paint.setColor(getTextHighColor());
            paint.setTextSize(dp(14));
            paint.setTextAlign(Paint.Align.LEFT);
            paint.setFakeBoldText(true);
            canvas.drawText(timeStr, dp(16), dp(26), paint);

            // Center: Dynamic Island
            float pillW = dp(84);
            float pillH = dp(24);
            float pillX = (w - pillW) / 2f;
            float pillY = dp(9);
            paint.setColor(darkMode ? 0xFF000000 : 0xFF2D1E47);
            canvas.drawRoundRect(pillX, pillY, pillX + pillW, pillY + pillH, pillH / 2f, pillH / 2f, paint);

            paint.setColor(0xFF1E293B);
            canvas.drawCircle(pillX + dp(18), pillY + pillH / 2f, dp(4), paint);
            paint.setColor(0xFF0F172A);
            canvas.drawCircle(pillX + pillW - dp(24), pillY + pillH / 2f, dp(5), paint);

            // Tapping dynamic island opens Control Center!
            touchAreas.add(new TouchArea(new RectF(0, 0, w, barHeight), "open_control_center"));

            // Right side indicators
            float rightX = w - dp(16);

            // 1. Dynamic Battery
            float batW = dp(24);
            float batH = dp(13);
            float batX = rightX - batW;
            float batY = dp(14);

            paint.setStyle(Paint.Style.STROKE);
            paint.setColor(getTextMedColor());
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

            rightX = batX - dp(10);

            // 2. Cellular Tower
            float towerW = dp(18);
            float towerBaseY = dp(27);
            float[] barHeights = {dp(3.5f), dp(6.5f), dp(9.5f), dp(13f)};

            paint.setStyle(Paint.Style.FILL);
            float towerX = rightX - towerW;
            if (cellularEnabled) {
                for (int i = 0; i < 4; i++) {
                    float bx = towerX + i * dp(4.5f);
                    float by = towerBaseY - barHeights[i];
                    paint.setColor(i < cellularSignalLevel ? getTextHighColor() : (darkMode ? 0xFF2D3748 : 0xFFCBD5E1));
                    canvas.drawRoundRect(bx, by, bx + dp(2.5f), towerBaseY, dp(1), dp(1), paint);
                }
            }

            rightX = towerX - dp(6);

            // Cellular Badge
            String netBadge = cellularEnabled ? (cellularSignalLevel >= 3 ? "৫G" : (cellularSignalLevel == 2 ? "৪G" : "৩G")) : "অফ";
            paint.setColor(cellularEnabled ? COLOR_CYAN : getTextDimColor());
            paint.setTextSize(dp(11));
            paint.setTextAlign(Paint.Align.RIGHT);
            paint.setFakeBoldText(true);
            canvas.drawText(netBadge, rightX, dp(25), paint);

            rightX -= dp(28);

            // 3. Wi-Fi Arcs
            float wifiW = dp(16);
            float wifiX = rightX - wifiW;

            if (wifiEnabled) {
                paint.setStyle(Paint.Style.FILL);
                paint.setColor(wifiSignalLevel >= 1 ? COLOR_CYAN : (darkMode ? 0xFF2D3748 : 0xFFCBD5E1));
                canvas.drawCircle(wifiX + wifiW / 2f, dp(26.5f), dp(1.8f), paint);

                paint.setStyle(Paint.Style.STROKE);
                paint.setStrokeWidth(dp(1.6f));
                paint.setColor(wifiSignalLevel >= 2 ? COLOR_CYAN : (darkMode ? 0xFF2D3748 : 0xFFCBD5E1));
                canvas.drawArc(new RectF(wifiX + dp(2.5f), dp(16.5f), wifiX + wifiW - dp(2.5f), dp(29)), 220, 100, false, paint);

                paint.setColor(wifiSignalLevel >= 3 ? COLOR_CYAN : (darkMode ? 0xFF2D3748 : 0xFFCBD5E1));
                canvas.drawArc(new RectF(wifiX, dp(11.5f), wifiX + wifiW, dp(29)), 220, 100, false, paint);
            }
        }

        // ─── Bottom Navigation Bar ──────────────────────────────────────────────────

        private void renderBottomNav(Canvas canvas, int w, int h) {
            int navH = dp(52);
            float navY = h - navH;

            paint.setStyle(Paint.Style.FILL);
            paint.setColor(darkMode ? 0xF01E1236 : 0xF0F5F0FF);
            canvas.drawRect(0, navY, w, h, paint);

            paint.setColor(getBorderColor());
            canvas.drawRect(0, navY, w, navY + dp(1), paint);

            // Left: Back button <
            RectF backRect = new RectF(dp(16), navY, dp(72), h);
            paint.setStyle(Paint.Style.FILL);
            paint.setColor(getTextHighColor());
            paint.setTextSize(dp(20));
            paint.setTextAlign(Paint.Align.CENTER);
            paint.setFakeBoldText(true);
            canvas.drawText("〈", backRect.centerX(), navY + dp(32), paint);
            touchAreas.add(new TouchArea(backRect, "nav_back"));

            // Center: Modern gesture pill ━━━━
            float pillW = dp(110);
            float pillH = dp(5);
            float px = (w - pillW) / 2f;
            float py = navY + dp(24);
            paint.setColor(getTextMedColor());
            canvas.drawRoundRect(px, py, px + pillW, py + pillH, pillH / 2f, pillH / 2f, paint);
            touchAreas.add(new TouchArea(new RectF(px - dp(20), navY, px + pillW + dp(20), h), "nav_home"));

            // Right: Recents / Overview icon ☷
            RectF recRect = new RectF(w - dp(72), navY, w - dp(16), h);
            paint.setTextSize(dp(18));
            canvas.drawText("☷", recRect.centerX(), navY + dp(32), paint);
            touchAreas.add(new TouchArea(recRect, "nav_recents"));
        }

        // ─── Screen 1: Lockscreen ───────────────────────────────────────────────────

        private void renderLockScreen(Canvas canvas, int w, int h) {
            float y = dp(80);

            SimpleDateFormat sdfTime = new SimpleDateFormat("HH:mm", Locale.getDefault());
            String timeStr = toBengaliDigits(sdfTime.format(new Date()));
            paint.setColor(getTextHighColor());
            paint.setTextSize(dp(64));
            paint.setTextAlign(Paint.Align.CENTER);
            paint.setFakeBoldText(true);
            canvas.drawText(timeStr, w / 2f, y + dp(50), paint);

            SimpleDateFormat sdfDate = new SimpleDateFormat("EEEE, d MMMM", new Locale("bn", "BD"));
            paint.setColor(COLOR_CYAN);
            paint.setTextSize(dp(16));
            paint.setFakeBoldText(false);
            canvas.drawText(sdfDate.format(new Date()), w / 2f, y + dp(84), paint);

            // Unlock prompt
            float cardY = h / 2f - dp(30);
            paint.setColor(getTextMedColor());
            paint.setTextSize(dp(14));
            canvas.drawText("আনলক করতে ট্যাপ করুন", w / 2f, cardY, paint);

            // Unlock button card
            RectF unlockCard = new RectF(dp(40), cardY + dp(16), w - dp(40), cardY + dp(70));
            paint.setColor(getSurfaceColor());
            canvas.drawRoundRect(unlockCard, dp(16), dp(16), paint);
            paint.setStyle(Paint.Style.STROKE);
            paint.setColor(getBorderColor());
            canvas.drawRoundRect(unlockCard, dp(16), dp(16), paint);

            paint.setStyle(Paint.Style.FILL);
            paint.setColor(COLOR_CYAN);
            paint.setTextSize(dp(16));
            paint.setFakeBoldText(true);
            canvas.drawText("🔓 সিস্টেম আনলক", w / 2f, cardY + dp(50), paint);
            touchAreas.add(new TouchArea(unlockCard, "unlock_device"));

            // Bottom shortcuts: Emergency call & Camera
            float btnY = h - dp(90);
            RectF emergRect = new RectF(dp(24), btnY, dp(140), btnY + dp(44));
            paint.setColor(getSurfaceColor());
            canvas.drawRoundRect(emergRect, dp(22), dp(22), paint);
            paint.setColor(COLOR_RED);
            paint.setTextSize(dp(13));
            canvas.drawText("📞 জরুরি কল", emergRect.centerX(), btnY + dp(28), paint);
            touchAreas.add(new TouchArea(emergRect, "lock_emergency"));

            RectF camRect = new RectF(w - dp(140), btnY, w - dp(24), btnY + dp(44));
            paint.setColor(getSurfaceColor());
            canvas.drawRoundRect(camRect, dp(22), dp(22), paint);
            paint.setColor(COLOR_PURPLE);
            canvas.drawText("📷 ক্যামেরা", camRect.centerX(), btnY + dp(28), paint);
            touchAreas.add(new TouchArea(camRect, "lock_camera"));
        }

        // ─── Screen 2: Home Screen (ArkUI/Material 3 Grid: 12 Apps) ─────────────────

        private void renderHomeScreen(Canvas canvas, int w, int h) {
            float startY = dp(56);

            // Hero Clock
            SimpleDateFormat sdfHour = new SimpleDateFormat("HH:mm", Locale.getDefault());
            String clockStr = toBengaliDigits(sdfHour.format(new Date()));
            paint.setColor(getTextHighColor());
            paint.setTextSize(dp(54));
            paint.setTextAlign(Paint.Align.CENTER);
            paint.setFakeBoldText(true);
            canvas.drawText(clockStr, w / 2f, startY + dp(50), paint);

            // Date in Bengali
            SimpleDateFormat sdfDate = new SimpleDateFormat("EEEE, d MMMM yyyy", new Locale("bn", "BD"));
            String dateStr = sdfDate.format(new Date());
            paint.setColor(COLOR_CYAN);
            paint.setTextSize(dp(14));
            paint.setFakeBoldText(false);
            canvas.drawText(dateStr, w / 2f, startY + dp(78), paint);

            // System Chip Card
            float chipY = startY + dp(96);
            paint.setStyle(Paint.Style.FILL);
            paint.setColor(getSurfaceColor());
            RectF chipRect = new RectF(dp(18), chipY, w - dp(18), chipY + dp(40));
            canvas.drawRoundRect(chipRect, dp(12), dp(12), paint);
            paint.setStyle(Paint.Style.STROKE);
            paint.setColor(getBorderColor());
            paint.setStrokeWidth(dp(1));
            canvas.drawRoundRect(chipRect, dp(12), dp(12), paint);

            paint.setStyle(Paint.Style.FILL);
            paint.setColor(getTextMedColor());
            paint.setTextSize(dp(11.5f));
            paint.setTextAlign(Paint.Align.CENTER);
            canvas.drawText("⚡ Snapdragon 8 Elite • Onuron OS 1.0 (Alap Engine)", w / 2f, chipY + dp(25), paint);

            // App Grid (3 rows x 4 columns = 12 apps)
            float gridY = chipY + dp(54);
            int cols = 4;
            float iconSize = dp(56);
            float colWidth = (w - dp(32)) / (float) cols;

            AppItem[] apps = {
                    new AppItem("ফোন", "app_phone", "📞", COLOR_GREEN),
                    new AppItem("ব্রাউজার", "app_browser", "🌐", COLOR_CYAN),
                    new AppItem("বার্তা", "app_messages", "💬", COLOR_AMBER),
                    new AppItem("ফাইল", "app_files", "📁", COLOR_BLUE),
                    new AppItem("ক্যামেরা", "app_camera", "📷", COLOR_PURPLE),
                    new AppItem("নোটস", "app_notes", "📝", COLOR_AMBER),
                    new AppItem("মিউজিক", "app_music", "🎵", COLOR_GREEN),
                    new AppItem("হিসাব", "app_calculator", "🧮", COLOR_CYAN),
                    new AppItem("সেটিংস", "app_settings", "⚙️", COLOR_PURPLE),
                    new AppItem("টার্মিনাল", "app_terminal", "💻", COLOR_CYAN),
                    new AppItem("সফটবাস", "app_softbus", "⚡", COLOR_GREEN),
                    new AppItem("নীলপ্যাক", "app_nilpkg", "📦", COLOR_CYAN),
            };

            for (int i = 0; i < apps.length; i++) {
                int r = i / cols;
                int c = i % cols;
                float cx = dp(16) + c * colWidth + colWidth / 2f;
                float cy = gridY + r * dp(90);

                RectF iconRect = new RectF(cx - iconSize / 2f, cy, cx + iconSize / 2f, cy + iconSize);
                paint.setStyle(Paint.Style.FILL);
                paint.setColor(getSurfaceColor());
                canvas.drawRoundRect(iconRect, dp(14), dp(14), paint);

                paint.setStyle(Paint.Style.STROKE);
                paint.setColor(getBorderColor());
                paint.setStrokeWidth(dp(1.2f));
                canvas.drawRoundRect(iconRect, dp(14), dp(14), paint);

                drawAppVectorGlyph(canvas, cx, cy + iconSize / 2f, apps[i].id, darkMode ? 0xFFD9C6FF : 0xFF7C3AED);

                paint.setStyle(Paint.Style.FILL);
                paint.setColor(getTextHighColor());
                paint.setTextSize(dp(12));
                paint.setFakeBoldText(false);
                canvas.drawText(apps[i].title, cx, cy + iconSize + dp(18), paint);

                touchAreas.add(new TouchArea(iconRect, apps[i].id));
            }
        }

        // ─── Screen 3: Control Center / Notification Panel (Screen 3) ───────────────

        private void renderControlCenter(Canvas canvas, int w, int h) {
            float y = dp(46);

            // Header date
            SimpleDateFormat sdfDate = new SimpleDateFormat("EEEE, d MMMM", new Locale("bn", "BD"));
            paint.setColor(getTextHighColor());
            paint.setTextSize(dp(20));
            paint.setFakeBoldText(true);
            paint.setTextAlign(Paint.Align.LEFT);
            canvas.drawText(sdfDate.format(new Date()), dp(24), y + dp(18), paint);

            // Close button ✕
            RectF closeBtn = new RectF(w - dp(56), y, w - dp(24), y + dp(34));
            paint.setStyle(Paint.Style.FILL);
            paint.setColor(getSurfaceColor());
            canvas.drawRoundRect(closeBtn, dp(17), dp(17), paint);
            paint.setStyle(Paint.Style.STROKE);
            paint.setColor(getBorderColor());
            canvas.drawRoundRect(closeBtn, dp(17), dp(17), paint);
            paint.setStyle(Paint.Style.FILL);
            paint.setColor(getTextMedColor());
            paint.setTextSize(dp(16));
            paint.setTextAlign(Paint.Align.CENTER);
            canvas.drawText("✕", closeBtn.centerX(), closeBtn.centerY() + dp(5), paint);
            touchAreas.add(new TouchArea(closeBtn, "close_control"));

            // 2x4 Quick Settings Tiles
            y += dp(48);
            String[][] qsTiles = {
                    {"ওয়াই-ফাই", wifiEnabled ? "চালু" : "বন্ধ", "toggle_wifi", wifiEnabled ? "1" : "0"},
                    {"ব্লুটুথ", btEnabled ? "চালু" : "বন্ধ", "toggle_bt", btEnabled ? "1" : "0"},
                    {"টর্চলাইট", torchEnabled ? "চালু" : "বন্ধ", "toggle_torch", torchEnabled ? "1" : "0"},
                    {"নীরব মোড", soundVolume == 0 ? "চালু" : "বন্ধ", "toggle_silent", soundVolume == 0 ? "1" : "0"},
                    {"রোটেশন", "লক", "toggle_rotation", "0"},
                    {"ফ্লাইট মোড", airplaneMode ? "চালু" : "বন্ধ", "toggle_airplane", airplaneMode ? "1" : "0"},
                    {"মোবাইল ডেটা", cellularEnabled ? "চালু" : "বন্ধ", "toggle_cellular", cellularEnabled ? "1" : "0"},
                    {"ডার্ক মোড", darkMode ? "চালু" : "বন্ধ", "toggle_dark_theme", darkMode ? "1" : "0"},
            };

            float tileW = (w - dp(48 + 24)) / 4f;
            float tileH = dp(52);

            for (int i = 0; i < qsTiles.length; i++) {
                int col = i % 4;
                int row = i / 4;
                float tx = dp(24) + col * (tileW + dp(8));
                float ty = y + row * (tileH + dp(8));
                RectF tRect = new RectF(tx, ty, tx + tileW, ty + tileH);

                boolean active = "1".equals(qsTiles[i][3]);
                paint.setStyle(Paint.Style.FILL);
                paint.setColor(active ? COLOR_PURPLE : getSurfaceColor());
                canvas.drawRoundRect(tRect, dp(14), dp(14), paint);

                if (!active) {
                    paint.setStyle(Paint.Style.STROKE);
                    paint.setColor(getBorderColor());
                    canvas.drawRoundRect(tRect, dp(14), dp(14), paint);
                }

                paint.setStyle(Paint.Style.FILL);
                paint.setColor(active ? 0xFFFFFFFF : getTextHighColor());
                paint.setTextSize(dp(11));
                paint.setFakeBoldText(true);
                paint.setTextAlign(Paint.Align.CENTER);
                canvas.drawText(qsTiles[i][0], tRect.centerX(), ty + dp(22), paint);

                paint.setColor(active ? 0xCCFFFFFF : getTextMedColor());
                paint.setTextSize(dp(9.5f));
                paint.setFakeBoldText(false);
                canvas.drawText(qsTiles[i][1], tRect.centerX(), ty + dp(38), paint);

                touchAreas.add(new TouchArea(tRect, qsTiles[i][2]));
            }

            // Brightness Slider Card
            y += dp(118);
            RectF brCard = new RectF(dp(24), y, w - dp(24), y + dp(50));
            paint.setStyle(Paint.Style.FILL);
            paint.setColor(getSurfaceColor());
            canvas.drawRoundRect(brCard, dp(14), dp(14), paint);
            paint.setStyle(Paint.Style.STROKE);
            paint.setColor(getBorderColor());
            canvas.drawRoundRect(brCard, dp(14), dp(14), paint);

            // Slider Fill Track
            float fillW = (brCard.width() - dp(80)) * (brightnessLevel / 100f);
            RectF fillRect = new RectF(dp(70), y + dp(18), dp(70) + fillW, y + dp(32));
            paint.setStyle(Paint.Style.FILL);
            paint.setColor(COLOR_PURPLE);
            canvas.drawRoundRect(fillRect, dp(7), dp(7), paint);

            paint.setColor(getTextHighColor());
            paint.setTextSize(dp(16));
            canvas.drawText("☀️", dp(44), y + dp(32), paint);

            paint.setTextSize(dp(12));
            paint.setTextAlign(Paint.Align.RIGHT);
            canvas.drawText(toBengaliDigits(String.valueOf(brightnessLevel)) + "%", w - dp(36), y + dp(31), paint);
            touchAreas.add(new TouchArea(brCard, "cycle_brightness"));

            // Notifications Section
            y += dp(66);
            paint.setColor(getTextHighColor());
            paint.setTextSize(dp(16));
            paint.setTextAlign(Paint.Align.LEFT);
            paint.setFakeBoldText(true);
            canvas.drawText("বিজ্ঞপ্তি (২)", dp(24), y + dp(18), paint);

            RectF clearBtn = new RectF(w - dp(100), y, w - dp(24), y + dp(28));
            paint.setColor(getSurfaceColor());
            canvas.drawRoundRect(clearBtn, dp(12), dp(12), paint);
            paint.setColor(COLOR_CYAN);
            paint.setTextSize(dp(11));
            paint.setTextAlign(Paint.Align.CENTER);
            paint.setFakeBoldText(false);
            canvas.drawText("সব মুছুন", clearBtn.centerX(), clearBtn.centerY() + dp(4), paint);
            touchAreas.add(new TouchArea(clearBtn, "clear_notifications"));

            // Notification Card 1
            y += dp(36);
            RectF notif1 = new RectF(dp(24), y, w - dp(24), y + dp(62));
            paint.setColor(getSurfaceColor());
            canvas.drawRoundRect(notif1, dp(14), dp(14), paint);
            paint.setStyle(Paint.Style.STROKE);
            paint.setColor(getBorderColor());
            canvas.drawRoundRect(notif1, dp(14), dp(14), paint);

            paint.setStyle(Paint.Style.FILL);
            paint.setColor(COLOR_PURPLE);
            paint.setTextSize(dp(12));
            paint.setFakeBoldText(true);
            paint.setTextAlign(Paint.Align.LEFT);
            canvas.drawText("💬 বার্তা • ৫ মিনিট আগে", dp(38), y + dp(24), paint);
            paint.setColor(getTextHighColor());
            paint.setTextSize(dp(13));
            paint.setFakeBoldText(false);
            canvas.drawText("অনিমেষ: কালকের সভার সময় কখন?", dp(38), y + dp(46), paint);
            touchAreas.add(new TouchArea(notif1, "notif_open_msg"));

            // Notification Card 2
            y += dp(72);
            RectF notif2 = new RectF(dp(24), y, w - dp(24), y + dp(62));
            paint.setColor(getSurfaceColor());
            canvas.drawRoundRect(notif2, dp(14), dp(14), paint);
            paint.setStyle(Paint.Style.STROKE);
            paint.setColor(getBorderColor());
            canvas.drawRoundRect(notif2, dp(14), dp(14), paint);

            paint.setStyle(Paint.Style.FILL);
            paint.setColor(COLOR_CYAN);
            paint.setTextSize(dp(12));
            paint.setFakeBoldText(true);
            paint.setTextAlign(Paint.Align.LEFT);
            canvas.drawText("🔄 সিস্টেম আপডেট • ১ ঘণ্টা আগে", dp(38), y + dp(24), paint);
            paint.setColor(getTextHighColor());
            paint.setTextSize(dp(13));
            paint.setFakeBoldText(false);
            canvas.drawText("অনুরণ ওএস v১.২ প্রস্তুত — ইনস্টল করুন", dp(38), y + dp(46), paint);
            touchAreas.add(new TouchArea(notif2, "notif_open_update"));
        }

        // ─── Screen 4: Recents (App Switcher) ────────────────────────────────────────

        private void renderRecentsScreen(Canvas canvas, int w, int h) {
            float y = dp(56);
            paint.setColor(getTextHighColor());
            paint.setTextSize(dp(22));
            paint.setTextAlign(Paint.Align.LEFT);
            paint.setFakeBoldText(true);
            canvas.drawText("সাম্প্রতিক অ্যাপস", dp(24), y + dp(20), paint);

            y += dp(40);
            for (int i = 0; i < recentsList.size(); i++) {
                String title = recentsList.get(i);
                RectF card = new RectF(dp(24), y, w - dp(24), y + dp(68));
                paint.setStyle(Paint.Style.FILL);
                paint.setColor(getSurfaceColor());
                canvas.drawRoundRect(card, dp(16), dp(16), paint);
                paint.setStyle(Paint.Style.STROKE);
                paint.setColor(getBorderColor());
                canvas.drawRoundRect(card, dp(16), dp(16), paint);

                paint.setStyle(Paint.Style.FILL);
                paint.setColor(COLOR_PURPLE);
                paint.setTextSize(dp(16));
                paint.setFakeBoldText(true);
                paint.setTextAlign(Paint.Align.LEFT);
                canvas.drawText("📱 " + title, dp(40), y + dp(32), paint);

                paint.setColor(getTextMedColor());
                paint.setTextSize(dp(12));
                paint.setFakeBoldText(false);
                canvas.drawText("চলমান প্রক্রিয়া • আলপ রানটাইম", dp(40), y + dp(52), paint);

                touchAreas.add(new TouchArea(card, "recent_open_" + i));
                y += dp(78);
            }

            // Bottom action pill: [ ✕ সব বন্ধ করুন ]
            float btnY = h - dp(120);
            RectF clearAll = new RectF(dp(40), btnY, w - dp(40), btnY + dp(48));
            paint.setStyle(Paint.Style.FILL);
            paint.setColor(COLOR_PURPLE);
            canvas.drawRoundRect(clearAll, dp(24), dp(24), paint);

            paint.setColor(0xFFFFFFFF);
            paint.setTextSize(dp(15));
            paint.setTextAlign(Paint.Align.CENTER);
            paint.setFakeBoldText(true);
            canvas.drawText("✕ সব বন্ধ করুন", clearAll.centerX(), btnY + dp(30), paint);
            touchAreas.add(new TouchArea(clearAll, "clear_all_recents"));
        }

        // ─── Screen 5: Power Menu ───────────────────────────────────────────────────

        private void renderPowerMenuScreen(Canvas canvas, int w, int h) {
            paint.setColor(0xCC000000);
            canvas.drawRect(0, 0, w, h, paint);

            float y = h / 2f - dp(140);
            paint.setColor(0xFFFFFFFF);
            paint.setTextSize(dp(20));
            paint.setTextAlign(Paint.Align.CENTER);
            paint.setFakeBoldText(true);
            canvas.drawText("অনুরণ ওএস — পাওয়ার মেনু", w / 2f, y, paint);

            String[] powerOptions = {
                    "🛑 বন্ধ করুন (Power Off)",
                    "🔄 রিস্টার্ট (Restart)",
                    "⚡ রিকভারি মোড (Recovery)",
                    "✕ বাতিল (Cancel)"
            };
            int[] colors = {COLOR_RED, COLOR_AMBER, COLOR_CYAN, 0xFF64748B};

            y += dp(24);
            for (int i = 0; i < powerOptions.length; i++) {
                RectF optCard = new RectF(dp(36), y, w - dp(36), y + dp(50));
                paint.setStyle(Paint.Style.FILL);
                paint.setColor(getSurfaceColor());
                canvas.drawRoundRect(optCard, dp(14), dp(14), paint);
                paint.setStyle(Paint.Style.STROKE);
                paint.setColor(getBorderColor());
                canvas.drawRoundRect(optCard, dp(14), dp(14), paint);

                paint.setStyle(Paint.Style.FILL);
                paint.setColor(colors[i]);
                paint.setTextSize(dp(15));
                paint.setFakeBoldText(true);
                paint.setTextAlign(Paint.Align.CENTER);
                canvas.drawText(powerOptions[i], optCard.centerX(), y + dp(31), paint);

                touchAreas.add(new TouchArea(optCard, "power_opt_" + i));
                y += dp(60);
            }
        }

        // ─── Screen 6: Phone / Dialer (Screen 6) ────────────────────────────────────

        private void renderPhoneScreen(Canvas canvas, int w, int h) {
            float y = dp(52);
            paint.setColor(COLOR_PURPLE);
            paint.setTextSize(dp(20));
            paint.setFakeBoldText(true);
            paint.setTextAlign(Paint.Align.LEFT);
            canvas.drawText("ফোন", dp(24), y + dp(18), paint);

            // Tab Bar: [ ডায়ালপ্যাড ] | [ সাম্প্রতিক ] | [ পরিচিতি ]
            y += dp(32);
            float tabW = (w - dp(48)) / 3f;
            String[] tabs = {"ডায়ালপ্যাড", "সাম্প্রতিক (" + callLogs.size() + ")", "পরিচিতি (" + contacts.size() + ")"};
            for (int i = 0; i < 3; i++) {
                RectF tRect = new RectF(dp(24) + i * tabW, y, dp(24) + (i + 1) * tabW - dp(6), y + dp(34));
                paint.setStyle(Paint.Style.FILL);
                paint.setColor(phoneTab == i ? COLOR_PURPLE : getSurfaceColor());
                canvas.drawRoundRect(tRect, dp(17), dp(17), paint);

                paint.setColor(phoneTab == i ? 0xFFFFFFFF : getTextMedColor());
                paint.setTextSize(dp(12));
                paint.setTextAlign(Paint.Align.CENTER);
                canvas.drawText(tabs[i], tRect.centerX(), y + dp(22), paint);

                touchAreas.add(new TouchArea(tRect, "phone_tab_" + i));
            }

            y += dp(44);

            if (phoneTab == 0) {
                // Recent call quick pill
                RectF recChip = new RectF(dp(24), y, w - dp(24), y + dp(34));
                paint.setColor(getSurfaceColor());
                canvas.drawRoundRect(recChip, dp(12), dp(12), paint);
                paint.setStyle(Paint.Style.STROKE);
                paint.setColor(getBorderColor());
                canvas.drawRoundRect(recChip, dp(12), dp(12), paint);

                paint.setStyle(Paint.Style.FILL);
                paint.setColor(COLOR_CYAN);
                paint.setTextSize(dp(11.5f));
                paint.setTextAlign(Paint.Align.CENTER);
                canvas.drawText("সাম্প্রতিক: অনিমেষ (১০:২৪)", w / 2f, y + dp(22), paint);

                // Number Display Card
                y += dp(42);
                RectF numCard = new RectF(dp(24), y, w - dp(24), y + dp(56));
                paint.setColor(getSurfaceColor());
                canvas.drawRoundRect(numCard, dp(16), dp(16), paint);
                paint.setStyle(Paint.Style.STROKE);
                paint.setColor(getBorderColor());
                canvas.drawRoundRect(numCard, dp(16), dp(16), paint);

                paint.setStyle(Paint.Style.FILL);
                paint.setColor(getTextHighColor());
                paint.setTextSize(dp(26));
                paint.setFakeBoldText(true);
                paint.setTextAlign(Paint.Align.CENTER);
                String display = dialNumber.isEmpty() ? "নম্বর লিখুন..." : toBengaliDigits(dialNumber);
                canvas.drawText(display, w / 2f, y + dp(38), paint);

                // 4x3 Keypad Pills
                float padStartY = y + dp(66);
                float btnW = (w - dp(72)) / 3f;
                float btnH = dp(48);
                String[][] pad = {
                        {"1", "2", "3"},
                        {"4", "5", "6"},
                        {"7", "8", "9"},
                        {"*", "0", "#"}
                };

                for (int r = 0; r < 4; r++) {
                    for (int c = 0; c < 3; c++) {
                        float bx = dp(24) + c * (btnW + dp(12));
                        float by = padStartY + r * (btnH + dp(10));
                        RectF btnRect = new RectF(bx, by, bx + btnW, by + btnH);

                        paint.setStyle(Paint.Style.FILL);
                        paint.setColor(getSurfaceColor());
                        canvas.drawRoundRect(btnRect, dp(14), dp(14), paint);
                        paint.setStyle(Paint.Style.STROKE);
                        paint.setColor(getBorderColor());
                        canvas.drawRoundRect(btnRect, dp(14), dp(14), paint);

                        paint.setStyle(Paint.Style.FILL);
                        paint.setColor(getTextHighColor());
                        paint.setTextSize(dp(20));
                        paint.setTextAlign(Paint.Align.CENTER);
                        canvas.drawText(toBengaliDigits(pad[r][c]), bx + btnW / 2f, by + btnH / 2f + dp(7), paint);

                        touchAreas.add(new TouchArea(btnRect, "key_" + pad[r][c]));
                    }
                }

                // Bottom row: [ 👤 পরিচিতি ]  [ 📞 কল ]  [ ⌫ মুছুন ]
                float actY = padStartY + 4 * (btnH + dp(10)) + dp(8);

                // Contacts icon
                RectF contactBtn = new RectF(dp(36), actY, dp(36 + 54), actY + dp(54));
                paint.setStyle(Paint.Style.FILL);
                paint.setColor(getSurfaceColor());
                canvas.drawRoundRect(contactBtn, dp(27), dp(27), paint);
                paint.setColor(COLOR_CYAN);
                paint.setTextSize(dp(20));
                canvas.drawText("👤", contactBtn.centerX(), actY + dp(34), paint);
                touchAreas.add(new TouchArea(contactBtn, "phone_tab_2"));

                // Big circular emerald green Call Button
                float callCX = w / 2f;
                float callCY = actY + dp(27);
                paint.setColor(COLOR_GREEN);
                canvas.drawCircle(callCX, callCY, dp(30), paint);
                paint.setColor(0xFFFFFFFF);
                paint.setTextSize(dp(26));
                canvas.drawText("📞", callCX, callCY + dp(9), paint);
                touchAreas.add(new TouchArea(new RectF(callCX - dp(32), callCY - dp(32), callCX + dp(32), callCY + dp(32)), "act_call_screen"));

                // Backspace icon
                RectF delBtn = new RectF(w - dp(36 + 54), actY, w - dp(36), actY + dp(54));
                paint.setColor(getSurfaceColor());
                canvas.drawRoundRect(delBtn, dp(27), dp(27), paint);
                paint.setColor(COLOR_RED);
                paint.setTextSize(dp(20));
                canvas.drawText("⌫", delBtn.centerX(), actY + dp(34), paint);
                touchAreas.add(new TouchArea(delBtn, "act_del"));

            } else if (phoneTab == 1) {
                // Call History List
                for (int i = 0; i < callLogs.size(); i++) {
                    String[] log = callLogs.get(i);
                    RectF card = new RectF(dp(24), y, w - dp(24), y + dp(58));
                    paint.setStyle(Paint.Style.FILL);
                    paint.setColor(getSurfaceColor());
                    canvas.drawRoundRect(card, dp(14), dp(14), paint);

                    paint.setColor(COLOR_GREEN);
                    paint.setTextSize(dp(16));
                    paint.setFakeBoldText(true);
                    paint.setTextAlign(Paint.Align.LEFT);
                    canvas.drawText(log[0], dp(40), y + dp(26), paint);

                    paint.setColor(getTextMedColor());
                    paint.setTextSize(dp(12));
                    paint.setFakeBoldText(false);
                    canvas.drawText(log[2] + " • " + toBengaliDigits(log[1]), dp(40), y + dp(46), paint);

                    touchAreas.add(new TouchArea(card, "redial_log_" + i));
                    y += dp(66);
                }
            } else {
                // Contacts List
                for (int i = 0; i < contacts.size(); i++) {
                    String[] c = contacts.get(i);
                    RectF card = new RectF(dp(24), y, w - dp(24), y + dp(58));
                    paint.setStyle(Paint.Style.FILL);
                    paint.setColor(getSurfaceColor());
                    canvas.drawRoundRect(card, dp(14), dp(14), paint);

                    paint.setColor(COLOR_PURPLE);
                    canvas.drawCircle(dp(48), y + dp(29), dp(18), paint);
                    paint.setColor(0xFFFFFFFF);
                    paint.setTextSize(dp(14));
                    paint.setTextAlign(Paint.Align.CENTER);
                    canvas.drawText(c[0].substring(0, 1), dp(48), y + dp(34), paint);

                    paint.setColor(getTextHighColor());
                    paint.setTextSize(dp(15));
                    paint.setFakeBoldText(true);
                    paint.setTextAlign(Paint.Align.LEFT);
                    canvas.drawText(c[0], dp(76), y + dp(26), paint);

                    paint.setColor(getTextMedColor());
                    paint.setTextSize(dp(12));
                    paint.setFakeBoldText(false);
                    canvas.drawText(toBengaliDigits(c[1]), dp(76), y + dp(46), paint);

                    touchAreas.add(new TouchArea(card, "call_contact_" + i));
                    y += dp(66);
                }
            }
        }

        // ─── Screen: In-Call Screen ─────────────────────────────────────────────────

        private void renderInCallScreen(Canvas canvas, int w, int h) {
            float y = dp(110);

            // Contact Name
            paint.setColor(getTextHighColor());
            paint.setTextSize(dp(28));
            paint.setFakeBoldText(true);
            paint.setTextAlign(Paint.Align.CENTER);
            canvas.drawText("অনিমেষ", w / 2f, y, paint);

            // Phone Number
            paint.setColor(COLOR_CYAN);
            paint.setTextSize(dp(16));
            paint.setFakeBoldText(false);
            canvas.drawText(toBengaliDigits("০১৭৮৯-১২৩৪৫৬"), w / 2f, y + dp(28), paint);

            // Timer
            int min = inCallSec / 60;
            int sec = inCallSec % 60;
            String timeStr = String.format(Locale.getDefault(), "%02d:%02d", min, sec);
            paint.setColor(COLOR_GREEN);
            paint.setTextSize(dp(22));
            paint.setFakeBoldText(true);
            canvas.drawText("কল চলছে... " + toBengaliDigits(timeStr), w / 2f, y + dp(68), paint);

            // Control Pills: [ 🔇 নিঃশব্দ ]  [ 🔊 স্পিকার ]  [ 🔢 কীপ্যাড ]
            float pillY = y + dp(130);
            float pillW = (w - dp(72)) / 3f;
            String[] inCallPills = {
                    inCallMuted ? "নীরব (চালু)" : "নিঃশব্দ",
                    inCallSpeaker ? "স্পিকার (চালু)" : "স্পিকার",
                    "কীপ্যাড"
            };
            String[] pillIds = {"incall_mute", "incall_speaker", "incall_keypad"};

            for (int i = 0; i < 3; i++) {
                float px = dp(24) + i * (pillW + dp(12));
                RectF pRect = new RectF(px, pillY, px + pillW, pillY + dp(48));
                paint.setStyle(Paint.Style.FILL);
                paint.setColor(getSurfaceColor());
                canvas.drawRoundRect(pRect, dp(14), dp(14), paint);
                paint.setStyle(Paint.Style.STROKE);
                paint.setColor(getBorderColor());
                canvas.drawRoundRect(pRect, dp(14), dp(14), paint);

                paint.setStyle(Paint.Style.FILL);
                paint.setColor(getTextHighColor());
                paint.setTextSize(dp(12.5f));
                paint.setTextAlign(Paint.Align.CENTER);
                canvas.drawText(inCallPills[i], pRect.centerX(), pillY + dp(29), paint);

                touchAreas.add(new TouchArea(pRect, pillIds[i]));
            }

            // Big Circular Red End Call Button 📴
            float endCY = pillY + dp(120);
            paint.setColor(COLOR_RED);
            canvas.drawCircle(w / 2f, endCY, dp(34), paint);
            paint.setColor(0xFFFFFFFF);
            paint.setTextSize(dp(28));
            canvas.drawText("📴", w / 2f, endCY + dp(10), paint);
            touchAreas.add(new TouchArea(new RectF(w / 2f - dp(36), endCY - dp(36), w / 2f + dp(36), endCY + dp(36)), "incall_end"));
        }

        // ─── Screen 7: Messages (Screen 7) ──────────────────────────────────────────

        private void renderMessagesScreen(Canvas canvas, int w, int h) {
            float y = dp(52);

            if (inActiveChat) {
                // Active Chat View Header
                RectF backBtn = new RectF(dp(20), y, dp(54), y + dp(34));
                paint.setColor(getSurfaceColor());
                canvas.drawRoundRect(backBtn, dp(12), dp(12), paint);
                paint.setColor(getTextHighColor());
                paint.setTextSize(dp(18));
                paint.setTextAlign(Paint.Align.CENTER);
                canvas.drawText("〈", backBtn.centerX(), y + dp(24), paint);
                touchAreas.add(new TouchArea(backBtn, "chat_back"));

                paint.setColor(getTextHighColor());
                paint.setTextSize(dp(18));
                paint.setFakeBoldText(true);
                paint.setTextAlign(Paint.Align.LEFT);
                canvas.drawText(activeRecipient, dp(64), y + dp(24), paint);

                y += dp(48);
                // Chat bubbles
                for (String[] m : activeChatMessages) {
                    boolean isMe = "me".equals(m[0]);
                    float bubbleW = (w - dp(64)) * 0.72f;
                    float bx = isMe ? (w - dp(24) - bubbleW) : dp(24);
                    RectF bRect = new RectF(bx, y, bx + bubbleW, y + dp(52));

                    paint.setStyle(Paint.Style.FILL);
                    paint.setColor(isMe ? COLOR_PURPLE : getSurfaceColor());
                    canvas.drawRoundRect(bRect, dp(14), dp(14), paint);

                    paint.setColor(isMe ? 0xFFFFFFFF : getTextHighColor());
                    paint.setTextSize(dp(13));
                    paint.setTextAlign(Paint.Align.LEFT);
                    canvas.drawText(m[1], bx + dp(14), y + dp(26), paint);

                    paint.setColor(isMe ? 0xCCFFFFFF : getTextMedColor());
                    paint.setTextSize(dp(10));
                    canvas.drawText(toBengaliDigits(m[2]), bx + dp(14), y + dp(44), paint);

                    y += dp(60);
                }

                return;
            }

            // Normal Messages Thread List
            paint.setColor(COLOR_PURPLE);
            paint.setTextSize(dp(20));
            paint.setFakeBoldText(true);
            paint.setTextAlign(Paint.Align.LEFT);
            canvas.drawText("বার্তা", dp(24), y + dp(18), paint);

            // Search pill
            y += dp(32);
            RectF searchPill = new RectF(dp(24), y, w - dp(24), y + dp(38));
            paint.setStyle(Paint.Style.FILL);
            paint.setColor(getSurfaceColor());
            canvas.drawRoundRect(searchPill, dp(19), dp(19), paint);
            paint.setStyle(Paint.Style.STROKE);
            paint.setColor(getBorderColor());
            canvas.drawRoundRect(searchPill, dp(19), dp(19), paint);

            paint.setStyle(Paint.Style.FILL);
            paint.setColor(getTextMedColor());
            paint.setTextSize(dp(12));
            paint.setTextAlign(Paint.Align.LEFT);
            canvas.drawText("🔍 বার্তা বা কন্টাক্ট খুঁজুন...", dp(40), y + dp(24), paint);

            // Threads
            y += dp(48);
            int[] avatarColors = {COLOR_PURPLE, COLOR_CYAN, COLOR_GREEN, COLOR_BLUE, COLOR_AMBER};
            for (int i = 0; i < messageThreads.size(); i++) {
                String[] t = messageThreads.get(i);
                RectF card = new RectF(dp(24), y, w - dp(24), y + dp(62));
                paint.setStyle(Paint.Style.FILL);
                paint.setColor(getSurfaceColor());
                canvas.drawRoundRect(card, dp(14), dp(14), paint);
                paint.setStyle(Paint.Style.STROKE);
                paint.setColor(getBorderColor());
                canvas.drawRoundRect(card, dp(14), dp(14), paint);

                // Avatar
                paint.setStyle(Paint.Style.FILL);
                paint.setColor(avatarColors[i % avatarColors.length]);
                canvas.drawCircle(dp(48), y + dp(31), dp(18), paint);
                paint.setColor(0xFFFFFFFF);
                paint.setTextSize(dp(14));
                paint.setTextAlign(Paint.Align.CENTER);
                canvas.drawText(t[0].substring(0, 1), dp(48), y + dp(36), paint);

                // Name & preview
                paint.setColor(getTextHighColor());
                paint.setTextSize(dp(15));
                paint.setFakeBoldText(true);
                paint.setTextAlign(Paint.Align.LEFT);
                canvas.drawText(t[0], dp(76), y + dp(26), paint);

                paint.setColor(getTextMedColor());
                paint.setTextSize(dp(12));
                paint.setFakeBoldText(false);
                canvas.drawText(t[1], dp(76), y + dp(48), paint);

                // Time
                paint.setColor(getTextDimColor());
                paint.setTextSize(dp(10.5f));
                paint.setTextAlign(Paint.Align.RIGHT);
                canvas.drawText(toBengaliDigits(t[2]), w - dp(36), y + dp(26), paint);

                touchAreas.add(new TouchArea(card, "open_chat_" + i));
                y += dp(70);
            }

            // Bottom action pill: [ + নতুন বার্তা ]
            float btnY = h - dp(116);
            RectF newMsg = new RectF(dp(48), btnY, w - dp(48), btnY + dp(46));
            paint.setStyle(Paint.Style.FILL);
            paint.setColor(COLOR_PURPLE);
            canvas.drawRoundRect(newMsg, dp(23), dp(23), paint);
            paint.setColor(0xFFFFFFFF);
            paint.setTextSize(dp(15));
            paint.setTextAlign(Paint.Align.CENTER);
            paint.setFakeBoldText(true);
            canvas.drawText("+ নতুন বার্তা", newMsg.centerX(), btnY + dp(28), paint);
            touchAreas.add(new TouchArea(newMsg, "msg_new_chat"));
        }

        // ─── Screen 8: Files (Screen 8) ─────────────────────────────────────────────

        private void renderFilesScreen(Canvas canvas, int w, int h) {
            float y = dp(52);
            paint.setColor(COLOR_BLUE);
            paint.setTextSize(dp(20));
            paint.setFakeBoldText(true);
            paint.setTextAlign(Paint.Align.LEFT);
            canvas.drawText("ফাইল ম্যানেজার", dp(24), y + dp(18), paint);

            // Breadcrumb path pill
            y += dp(32);
            RectF pathPill = new RectF(dp(24), y, w - dp(24), y + dp(34));
            paint.setStyle(Paint.Style.FILL);
            paint.setColor(getSurfaceColor());
            canvas.drawRoundRect(pathPill, dp(12), dp(12), paint);
            paint.setColor(COLOR_CYAN);
            paint.setTextSize(dp(12));
            paint.setTextAlign(Paint.Align.LEFT);
            canvas.drawText("📂 /data/storage", dp(36), y + dp(22), paint);

            // Quick Category Pills
            y += dp(42);
            String[] cats = {"সব", "ডকুমেন্টস", "ছবি", "অডিও", "ডাউনলোড"};
            float pillW = (w - dp(48 + 24)) / 5f;
            for (int i = 0; i < cats.length; i++) {
                float px = dp(24) + i * (pillW + dp(6));
                RectF cp = new RectF(px, y, px + pillW, y + dp(28));
                paint.setStyle(Paint.Style.FILL);
                paint.setColor(filesFilter == i ? COLOR_PURPLE : getSurfaceColor());
                canvas.drawRoundRect(cp, dp(14), dp(14), paint);
                paint.setColor(filesFilter == i ? 0xFFFFFFFF : getTextMedColor());
                paint.setTextSize(dp(10.5f));
                paint.setTextAlign(Paint.Align.CENTER);
                canvas.drawText(cats[i], cp.centerX(), y + dp(18), paint);
                touchAreas.add(new TouchArea(cp, "file_cat_" + i));
            }

            // File items
            y += dp(38);
            String[][] fileItems = {
                    {"📁 Documents", "১২ আইটেম • ৪.৫ MB"},
                    {"📁 Download", "২৮ আইটেম • ১২৮ MB"},
                    {"📁 Pictures", "১৫৬ আইটেম • ৫১২ MB"},
                    {"📁 Music", "৪৫ আইটেম • ২৫০ MB"},
                    {"📄 system_log.txt", "১২ KB • আজ ১০:১৫"},
                    {"📄 config.json", "৪ KB • গতকাল"},
            };

            for (int i = 0; i < fileItems.length; i++) {
                RectF card = new RectF(dp(24), y, w - dp(24), y + dp(52));
                paint.setStyle(Paint.Style.FILL);
                paint.setColor(getSurfaceColor());
                canvas.drawRoundRect(card, dp(14), dp(14), paint);
                paint.setStyle(Paint.Style.STROKE);
                paint.setColor(getBorderColor());
                canvas.drawRoundRect(card, dp(14), dp(14), paint);

                paint.setStyle(Paint.Style.FILL);
                paint.setColor(getTextHighColor());
                paint.setTextSize(dp(14));
                paint.setFakeBoldText(true);
                paint.setTextAlign(Paint.Align.LEFT);
                canvas.drawText(fileItems[i][0], dp(40), y + dp(24), paint);

                paint.setColor(getTextMedColor());
                paint.setTextSize(dp(11.5f));
                paint.setFakeBoldText(false);
                canvas.drawText(toBengaliDigits(fileItems[i][1]), dp(40), y + dp(42), paint);

                touchAreas.add(new TouchArea(card, "file_item_" + i));
                y += dp(58);
            }
        }

        // ─── Screen 10: Camera (Screen 10) ──────────────────────────────────────────

        private void renderCameraScreen(Canvas canvas, int w, int h) {
            // Dark Viewfinder Canvas
            paint.setStyle(Paint.Style.FILL);
            paint.setColor(0xFF0F0B18);
            canvas.drawRect(0, 0, w, h, paint);

            // Rule of thirds grid lines
            paint.setColor(0x33FFFFFF);
            paint.setStrokeWidth(dp(1));
            canvas.drawLine(w / 3f, 0, w / 3f, h - dp(130), paint);
            canvas.drawLine(2 * w / 3f, 0, 2 * w / 3f, h - dp(130), paint);
            canvas.drawLine(0, (h - dp(130)) / 3f, w, (h - dp(130)) / 3f, paint);
            canvas.drawLine(0, 2 * (h - dp(130)) / 3f, w, 2 * (h - dp(130)) / 3f, paint);

            // Top Camera Pills: Flash & HDR
            float topY = dp(46);
            RectF flashPill = new RectF(dp(24), topY, dp(100), topY + dp(32));
            paint.setStyle(Paint.Style.FILL);
            paint.setColor(cameraFlash ? COLOR_AMBER : 0x66000000);
            canvas.drawRoundRect(flashPill, dp(16), dp(16), paint);
            paint.setColor(0xFFFFFFFF);
            paint.setTextSize(dp(12));
            paint.setTextAlign(Paint.Align.CENTER);
            canvas.drawText(cameraFlash ? "⚡ ফ্ল্যাশ" : "⚡ অফ", flashPill.centerX(), topY + dp(21), paint);
            touchAreas.add(new TouchArea(flashPill, "camera_toggle_flash"));

            // Zoom Selector Pill: [ 0.5x | 1x | 2x ]
            float zoomY = h - dp(210);
            RectF zoomCard = new RectF(w / 2f - dp(75), zoomY, w / 2f + dp(75), zoomY + dp(32));
            paint.setColor(0x88000000);
            canvas.drawRoundRect(zoomCard, dp(16), dp(16), paint);

            String[] zooms = {"0.5x", "1x", "2x"};
            float zw = zoomCard.width() / 3f;
            for (int i = 0; i < 3; i++) {
                float zx = zoomCard.left + i * zw;
                if (cameraZoom == i) {
                    paint.setColor(COLOR_PURPLE);
                    canvas.drawRoundRect(zx, zoomY, zx + zw, zoomY + dp(32), dp(16), dp(16), paint);
                }
                paint.setColor(0xFFFFFFFF);
                paint.setTextSize(dp(12));
                paint.setTextAlign(Paint.Align.CENTER);
                canvas.drawText(zooms[i], zx + zw / 2f, zoomY + dp(21), paint);
                touchAreas.add(new TouchArea(new RectF(zx, zoomY, zx + zw, zoomY + dp(32)), "camera_zoom_" + i));
            }

            // Mode selector: [ ছবি ] / [ ভিডিও ]
            float modeY = h - dp(168);
            paint.setColor(cameraMode == 0 ? COLOR_CYAN : 0x88FFFFFF);
            paint.setTextSize(dp(14));
            paint.setFakeBoldText(cameraMode == 0);
            paint.setTextAlign(Paint.Align.CENTER);
            canvas.drawText("ছবি", w / 2f - dp(40), modeY, paint);
            touchAreas.add(new TouchArea(new RectF(w / 2f - dp(70), modeY - dp(20), w / 2f - dp(10), modeY + dp(10)), "camera_mode_0"));

            paint.setColor(cameraMode == 1 ? COLOR_CYAN : 0x88FFFFFF);
            paint.setFakeBoldText(cameraMode == 1);
            canvas.drawText("ভিডিও", w / 2f + dp(40), modeY, paint);
            touchAreas.add(new TouchArea(new RectF(w / 2f + dp(10), modeY - dp(20), w / 2f + dp(70), modeY + dp(10)), "camera_mode_1"));

            // Bottom controls: Gallery thumbnail, Shutter Button, Flip camera
            float cY = h - dp(95);

            // Gallery thumbnail (Left)
            RectF galRect = new RectF(dp(36), cY - dp(24), dp(36 + 48), cY + dp(24));
            paint.setStyle(Paint.Style.FILL);
            paint.setColor(0xFF322748);
            canvas.drawRoundRect(galRect, dp(12), dp(12), paint);
            paint.setColor(0xFFFFFFFF);
            paint.setTextSize(dp(20));
            canvas.drawText("🖼️", galRect.centerX(), cY + dp(7), paint);
            touchAreas.add(new TouchArea(galRect, "camera_gallery"));

            // Big circular white shutter button (Center)
            paint.setStyle(Paint.Style.STROKE);
            paint.setColor(0xFFFFFFFF);
            paint.setStrokeWidth(dp(3));
            canvas.drawCircle(w / 2f, cY, dp(34), paint);

            paint.setStyle(Paint.Style.FILL);
            canvas.drawCircle(w / 2f, cY, dp(28), paint);
            touchAreas.add(new TouchArea(new RectF(w / 2f - dp(36), cY - dp(36), w / 2f + dp(36), cY + dp(36)), "camera_shutter"));

            // Flip camera button (Right)
            RectF flipRect = new RectF(w - dp(36 + 48), cY - dp(24), w - dp(36), cY + dp(24));
            paint.setColor(0xFF322748);
            canvas.drawRoundRect(flipRect, dp(12), dp(12), paint);
            paint.setColor(0xFFFFFFFF);
            paint.setTextSize(dp(20));
            canvas.drawText("🔄", flipRect.centerX(), cY + dp(7), paint);
            touchAreas.add(new TouchArea(flipRect, "camera_flip"));
        }

        // ─── Screen 12: Notes (Screen 12) ───────────────────────────────────────────

        private void renderNotesScreen(Canvas canvas, int w, int h) {
            float y = dp(52);
            paint.setColor(COLOR_AMBER);
            paint.setTextSize(dp(20));
            paint.setFakeBoldText(true);
            paint.setTextAlign(Paint.Align.LEFT);
            canvas.drawText("নোট", dp(24), y + dp(18), paint);

            // Search pill
            y += dp(32);
            RectF searchPill = new RectF(dp(24), y, w - dp(24), y + dp(38));
            paint.setStyle(Paint.Style.FILL);
            paint.setColor(getSurfaceColor());
            canvas.drawRoundRect(searchPill, dp(19), dp(19), paint);
            paint.setStyle(Paint.Style.STROKE);
            paint.setColor(getBorderColor());
            canvas.drawRoundRect(searchPill, dp(19), dp(19), paint);

            paint.setStyle(Paint.Style.FILL);
            paint.setColor(getTextMedColor());
            paint.setTextSize(dp(12));
            paint.setTextAlign(Paint.Align.LEFT);
            canvas.drawText("🔍 নোট খুঁজুন...", dp(40), y + dp(24), paint);

            // Filter pills: [ সব ]  [ কাজ ]  [ ব্যক্তিগত ]  [ জরুরি ]
            y += dp(46);
            String[] filters = {"সব", "কাজ", "ব্যক্তিগত", "জরুরি"};
            float fW = (w - dp(48 + 18)) / 4f;
            for (int i = 0; i < filters.length; i++) {
                float fx = dp(24) + i * (fW + dp(6));
                RectF fp = new RectF(fx, y, fx + fW, y + dp(30));
                paint.setStyle(Paint.Style.FILL);
                paint.setColor(notesCategory == i ? COLOR_PURPLE : getSurfaceColor());
                canvas.drawRoundRect(fp, dp(15), dp(15), paint);

                paint.setColor(notesCategory == i ? 0xFFFFFFFF : getTextMedColor());
                paint.setTextSize(dp(11.5f));
                paint.setTextAlign(Paint.Align.CENTER);
                canvas.drawText(filters[i], fp.centerX(), y + dp(19), paint);

                touchAreas.add(new TouchArea(fp, "notes_filter_" + i));
            }

            // Note cards
            y += dp(42);
            int[] badgeColors = {COLOR_RED, COLOR_BLUE, COLOR_PURPLE, COLOR_GREEN};
            for (int i = 0; i < notesList.size(); i++) {
                String[] n = notesList.get(i);
                RectF card = new RectF(dp(24), y, w - dp(24), y + dp(68));
                paint.setStyle(Paint.Style.FILL);
                paint.setColor(getSurfaceColor());
                canvas.drawRoundRect(card, dp(16), dp(16), paint);
                paint.setStyle(Paint.Style.STROKE);
                paint.setColor(getBorderColor());
                canvas.drawRoundRect(card, dp(16), dp(16), paint);

                // Priority Badge
                RectF badge = new RectF(dp(40), y + dp(14), dp(40 + 44), y + dp(32));
                paint.setStyle(Paint.Style.FILL);
                paint.setColor(badgeColors[i % badgeColors.length]);
                canvas.drawRoundRect(badge, dp(9), dp(9), paint);
                paint.setColor(0xFFFFFFFF);
                paint.setTextSize(dp(10));
                paint.setTextAlign(Paint.Align.CENTER);
                canvas.drawText(n[2], badge.centerX(), y + dp(26), paint);

                // Title
                paint.setColor(getTextHighColor());
                paint.setTextSize(dp(15));
                paint.setFakeBoldText(true);
                paint.setTextAlign(Paint.Align.LEFT);
                canvas.drawText(n[0], dp(94), y + dp(26), paint);

                // Snippet
                paint.setColor(getTextMedColor());
                paint.setTextSize(dp(12));
                paint.setFakeBoldText(false);
                canvas.drawText(n[1], dp(40), y + dp(50), paint);

                // Date
                paint.setColor(getTextDimColor());
                paint.setTextSize(dp(10.5f));
                paint.setTextAlign(Paint.Align.RIGHT);
                canvas.drawText(toBengaliDigits(n[3]), w - dp(36), y + dp(26), paint);

                touchAreas.add(new TouchArea(card, "notes_open_" + i));
                y += dp(78);
            }

            // Floating action pill: [ + নতুন নোট ]
            float btnY = h - dp(116);
            RectF addNote = new RectF(dp(48), btnY, w - dp(48), btnY + dp(46));
            paint.setStyle(Paint.Style.FILL);
            paint.setColor(COLOR_PURPLE);
            canvas.drawRoundRect(addNote, dp(23), dp(23), paint);
            paint.setColor(0xFFFFFFFF);
            paint.setTextSize(dp(15));
            paint.setTextAlign(Paint.Align.CENTER);
            paint.setFakeBoldText(true);
            canvas.drawText("+ নতুন নোট", addNote.centerX(), btnY + dp(28), paint);
            touchAreas.add(new TouchArea(addNote, "notes_add_new"));
        }

        // ─── Screen 13: Calculator (Screen 13) ──────────────────────────────────────

        private void renderCalculatorScreen(Canvas canvas, int w, int h) {
            float y = dp(52);
            paint.setColor(COLOR_CYAN);
            paint.setTextSize(dp(20));
            paint.setFakeBoldText(true);
            paint.setTextAlign(Paint.Align.LEFT);
            canvas.drawText("ক্যালকুলেটর", dp(24), y + dp(18), paint);

            // Formula expression display
            y += dp(40);
            paint.setColor(getTextMedColor());
            paint.setTextSize(dp(18));
            paint.setTextAlign(Paint.Align.RIGHT);
            canvas.drawText(calcFormula, w - dp(24), y, paint);

            // Big Bold Result display
            y += dp(42);
            paint.setColor(getTextHighColor());
            paint.setTextSize(dp(44));
            paint.setFakeBoldText(true);
            canvas.drawText(calcDisplay, w - dp(24), y, paint);

            // Top scientific row pills: [ sin ] [ cos ] [ √ ] [ π ]
            y += dp(24);
            String[] sci = {"sin", "cos", "√", "π"};
            float sciW = (w - dp(48 + 24)) / 4f;
            for (int i = 0; i < 4; i++) {
                float sx = dp(24) + i * (sciW + dp(8));
                RectF sRect = new RectF(sx, y, sx + sciW, y + dp(32));
                paint.setStyle(Paint.Style.FILL);
                paint.setColor(getSurfaceColor());
                canvas.drawRoundRect(sRect, dp(16), dp(16), paint);
                paint.setStyle(Paint.Style.STROKE);
                paint.setColor(getBorderColor());
                canvas.drawRoundRect(sRect, dp(16), dp(16), paint);

                paint.setStyle(Paint.Style.FILL);
                paint.setColor(COLOR_CYAN);
                paint.setTextSize(dp(13));
                paint.setTextAlign(Paint.Align.CENTER);
                canvas.drawText(sci[i], sRect.centerX(), y + dp(21), paint);

                touchAreas.add(new TouchArea(sRect, "calc_sci_" + sci[i]));
            }

            // 4x5 Circular Keypad
            y += dp(42);
            String[][] calcGrid = {
                    {"C", "()", "%", "÷"},
                    {"7", "8", "9", "×"},
                    {"4", "5", "6", "−"},
                    {"1", "2", "3", "+"},
                    {"±", "0", ".", "="}
            };

            float keyRadius = dp(28);
            float stepX = (w - dp(48)) / 4f;
            float stepY = dp(62);

            for (int r = 0; r < 5; r++) {
                for (int c = 0; c < 4; c++) {
                    float kcx = dp(24) + c * stepX + stepX / 2f;
                    float kcy = y + r * stepY + stepY / 2f;
                    String val = calcGrid[r][c];

                    paint.setStyle(Paint.Style.FILL);
                    if ("=".equals(val)) {
                        paint.setColor(COLOR_PURPLE);
                    } else if ("÷".equals(val) || "×".equals(val) || "−".equals(val) || "+".equals(val)) {
                        paint.setColor(getSurfaceAltColor());
                    } else if ("C".equals(val)) {
                        paint.setColor(darkMode ? 0xFF4A2838 : 0xFFFFE4E6);
                    } else {
                        paint.setColor(getSurfaceColor());
                    }
                    canvas.drawCircle(kcx, kcy, keyRadius, paint);

                    paint.setStyle(Paint.Style.STROKE);
                    paint.setColor(getBorderColor());
                    canvas.drawCircle(kcx, kcy, keyRadius, paint);

                    // Text
                    paint.setStyle(Paint.Style.FILL);
                    if ("=".equals(val)) {
                        paint.setColor(0xFFFFFFFF);
                    } else if ("C".equals(val)) {
                        paint.setColor(COLOR_RED);
                    } else if ("÷".equals(val) || "×".equals(val) || "−".equals(val) || "+".equals(val)) {
                        paint.setColor(COLOR_PURPLE);
                    } else {
                        paint.setColor(getTextHighColor());
                    }
                    paint.setTextSize(dp(20));
                    paint.setFakeBoldText(true);
                    paint.setTextAlign(Paint.Align.CENTER);
                    canvas.drawText(toBengaliDigits(val), kcx, kcy + dp(7), paint);

                    touchAreas.add(new TouchArea(new RectF(kcx - keyRadius, kcy - keyRadius, kcx + keyRadius, kcy + keyRadius), "calc_btn_" + val));
                }
            }
        }

        // ─── Screen 14: Music (Screen 14) ───────────────────────────────────────────

        private void renderMusicScreen(Canvas canvas, int w, int h) {
            float y = dp(52);
            paint.setColor(COLOR_GREEN);
            paint.setTextSize(dp(20));
            paint.setFakeBoldText(true);
            paint.setTextAlign(Paint.Align.LEFT);
            canvas.drawText("সঙ্গীত প্লেয়ার", dp(24), y + dp(18), paint);

            // Album art card
            y += dp(36);
            float artSize = w - dp(64);
            RectF artCard = new RectF(dp(32), y, w - dp(32), y + dp(170));
            paint.setStyle(Paint.Style.FILL);
            paint.setColor(getSurfaceColor());
            canvas.drawRoundRect(artCard, dp(20), dp(20), paint);

            // Vinyl glyph inside
            paint.setColor(COLOR_PURPLE);
            canvas.drawCircle(artCard.centerX(), artCard.centerY(), dp(45), paint);
            paint.setColor(getBgColor());
            canvas.drawCircle(artCard.centerX(), artCard.centerY(), dp(16), paint);
            paint.setColor(COLOR_CYAN);
            canvas.drawCircle(artCard.centerX(), artCard.centerY(), dp(6), paint);

            // Title & Artist
            y += dp(186);
            String[] track = musicTracks.get(musicCurrentTrack);
            paint.setColor(getTextHighColor());
            paint.setTextSize(dp(18));
            paint.setFakeBoldText(true);
            paint.setTextAlign(Paint.Align.CENTER);
            canvas.drawText(track[0], w / 2f, y, paint);

            paint.setColor(COLOR_CYAN);
            paint.setTextSize(dp(13));
            paint.setFakeBoldText(false);
            canvas.drawText(track[1], w / 2f, y + dp(22), paint);

            // Progress Bar
            y += dp(42);
            RectF barBg = new RectF(dp(32), y, w - dp(32), y + dp(6));
            paint.setColor(getSurfaceAltColor());
            canvas.drawRoundRect(barBg, dp(3), dp(3), paint);

            float progPct = Math.min(1.0f, musicProgressSec / 230f);
            RectF progFill = new RectF(dp(32), y, dp(32) + (w - dp(64)) * progPct, y + dp(6));
            paint.setColor(COLOR_PURPLE);
            canvas.drawRoundRect(progFill, dp(3), dp(3), paint);

            int curM = musicProgressSec / 60;
            int curS = musicProgressSec % 60;
            paint.setColor(getTextMedColor());
            paint.setTextSize(dp(10.5f));
            paint.setTextAlign(Paint.Align.LEFT);
            canvas.drawText(toBengaliDigits(String.format(Locale.getDefault(), "%02d:%02d", curM, curS)), dp(32), y + dp(18), paint);
            paint.setTextAlign(Paint.Align.RIGHT);
            canvas.drawText(toBengaliDigits("০৩:৫০"), w - dp(32), y + dp(18), paint);

            // Playback controls: [⏮]  [⏯]  [⏭]
            y += dp(36);
            float cY = y + dp(24);

            // Prev
            paint.setColor(getTextHighColor());
            paint.setTextSize(dp(24));
            paint.setTextAlign(Paint.Align.CENTER);
            canvas.drawText("⏮", w / 2f - dp(70), cY + dp(8), paint);
            touchAreas.add(new TouchArea(new RectF(w / 2f - dp(95), cY - dp(20), w / 2f - dp(45), cY + dp(20)), "music_prev"));

            // Play/Pause circular
            paint.setColor(COLOR_PURPLE);
            canvas.drawCircle(w / 2f, cY, dp(28), paint);
            paint.setColor(0xFFFFFFFF);
            paint.setTextSize(dp(22));
            canvas.drawText(musicPlaying ? "⏸" : "▶", w / 2f + (musicPlaying ? 0 : dp(2)), cY + dp(8), paint);
            touchAreas.add(new TouchArea(new RectF(w / 2f - dp(30), cY - dp(30), w / 2f + dp(30), cY + dp(30)), "music_toggle_play"));

            // Next
            paint.setColor(getTextHighColor());
            paint.setTextSize(dp(24));
            canvas.drawText("⏭", w / 2f + dp(70), cY + dp(8), paint);
            touchAreas.add(new TouchArea(new RectF(w / 2f + dp(45), cY - dp(20), w / 2f + dp(95), cY + dp(20)), "music_next"));
        }

        // ─── Screen 15: Terminal (Screen 15) ────────────────────────────────────────

        private void renderTerminalScreen(Canvas canvas, int w, int h) {
            float y = dp(52);
            paint.setColor(COLOR_CYAN);
            paint.setTextSize(dp(20));
            paint.setFakeBoldText(true);
            paint.setTextAlign(Paint.Align.LEFT);
            canvas.drawText("টার্মিনাল (Alap CLI)", dp(24), y + dp(18), paint);

            y += dp(36);
            RectF termBox = new RectF(dp(16), y, w - dp(16), h - dp(110));
            paint.setStyle(Paint.Style.FILL);
            paint.setColor(0xFF0B0814);
            canvas.drawRoundRect(termBox, dp(14), dp(14), paint);
            paint.setStyle(Paint.Style.STROKE);
            paint.setColor(getBorderColor());
            canvas.drawRoundRect(termBox, dp(14), dp(14), paint);

            float ty = y + dp(24);
            paint.setStyle(Paint.Style.FILL);
            paint.setTextSize(dp(11.5f));
            paint.setFakeBoldText(false);

            for (int i = Math.max(0, termLines.size() - 14); i < termLines.size(); i++) {
                String line = termLines.get(i);
                if (line.startsWith(">")) {
                    paint.setColor(0xFF00E676);
                } else if (line.contains("Error") || line.contains("ত্রুটি")) {
                    paint.setColor(COLOR_RED);
                } else {
                    paint.setColor(0xFFD4C8EA);
                }
                canvas.drawText(line, dp(28), ty, paint);
                ty += dp(18);
            }
        }

        // ─── Screen 16: SoftBus (Screen 16) ─────────────────────────────────────────

        private void renderSoftBusScreen(Canvas canvas, int w, int h) {
            float y = dp(52);
            paint.setColor(COLOR_GREEN);
            paint.setTextSize(dp(20));
            paint.setFakeBoldText(true);
            paint.setTextAlign(Paint.Align.LEFT);
            canvas.drawText("সফটবাস — ডিসট্রিবিউটেড নেটওয়ার্ক", dp(24), y + dp(18), paint);

            // Radar Scan Pulse Indicator
            y += dp(36);
            RectF radarCard = new RectF(dp(24), y, w - dp(24), y + dp(90));
            paint.setStyle(Paint.Style.FILL);
            paint.setColor(getSurfaceColor());
            canvas.drawRoundRect(radarCard, dp(16), dp(16), paint);

            paint.setColor(COLOR_GREEN);
            canvas.drawCircle(radarCard.centerX(), radarCard.centerY(), dp(24), paint);
            paint.setStyle(Paint.Style.STROKE);
            paint.setColor(0x8810B981);
            canvas.drawCircle(radarCard.centerX(), radarCard.centerY(), dp(36), paint);

            paint.setStyle(Paint.Style.FILL);
            paint.setColor(0xFFFFFFFF);
            paint.setTextSize(dp(12));
            paint.setTextAlign(Paint.Align.CENTER);
            canvas.drawText("সক্রিয় নোড খুঁজছে...", radarCard.centerX(), radarCard.bottom - dp(10), paint);

            // Device 1
            y += dp(104);
            RectF dev1 = new RectF(dp(24), y, w - dp(24), y + dp(62));
            paint.setColor(getSurfaceColor());
            canvas.drawRoundRect(dev1, dp(14), dp(14), paint);
            paint.setStyle(Paint.Style.STROKE);
            paint.setColor(getBorderColor());
            canvas.drawRoundRect(dev1, dp(14), dp(14), paint);

            paint.setStyle(Paint.Style.FILL);
            paint.setColor(getTextHighColor());
            paint.setTextSize(dp(15));
            paint.setFakeBoldText(true);
            paint.setTextAlign(Paint.Align.LEFT);
            canvas.drawText("💻 NilLaptop-Dev", dp(40), y + dp(26), paint);
            paint.setColor(getTextMedColor());
            paint.setTextSize(dp(11.5f));
            paint.setFakeBoldText(false);
            canvas.drawText("192.168.1.105 • P2P সংযুক্ত", dp(40), y + dp(46), paint);

            // Send file button
            RectF sendBtn = new RectF(w - dp(116), y + dp(16), w - dp(36), y + dp(46));
            paint.setColor(COLOR_PURPLE);
            canvas.drawRoundRect(sendBtn, dp(12), dp(12), paint);
            paint.setColor(0xFFFFFFFF);
            paint.setTextSize(dp(11));
            paint.setTextAlign(Paint.Align.CENTER);
            canvas.drawText("ফাইল পাঠান", sendBtn.centerX(), sendBtn.centerY() + dp(4), paint);
            touchAreas.add(new TouchArea(sendBtn, "softbus_send"));

            // Device 2
            y += dp(72);
            RectF dev2 = new RectF(dp(24), y, w - dp(24), y + dp(62));
            paint.setColor(getSurfaceColor());
            canvas.drawRoundRect(dev2, dp(14), dp(14), paint);

            paint.setColor(getTextHighColor());
            paint.setTextSize(dp(15));
            paint.setFakeBoldText(true);
            paint.setTextAlign(Paint.Align.LEFT);
            canvas.drawText("📱 NilPhone-S25 (এই ডিভাইস)", dp(40), y + dp(26), paint);
            paint.setColor(COLOR_GREEN);
            paint.setTextSize(dp(11.5f));
            paint.setFakeBoldText(false);
            canvas.drawText("মাস্টার কন্ট্রোলার সক্রিয়", dp(40), y + dp(46), paint);
        }

        // ─── Screen 17: NilPkg Store (Screen 17) ────────────────────────────────────

        private void renderNilPkgScreen(Canvas canvas, int w, int h) {
            float y = dp(52);
            paint.setColor(COLOR_CYAN);
            paint.setTextSize(dp(20));
            paint.setFakeBoldText(true);
            paint.setTextAlign(Paint.Align.LEFT);
            canvas.drawText("নীলপ্যাকেজ স্টোর (.nilax)", dp(24), y + dp(18), paint);

            // Search pill
            y += dp(32);
            RectF searchPill = new RectF(dp(24), y, w - dp(24), y + dp(38));
            paint.setStyle(Paint.Style.FILL);
            paint.setColor(getSurfaceColor());
            canvas.drawRoundRect(searchPill, dp(19), dp(19), paint);

            paint.setColor(getTextMedColor());
            paint.setTextSize(dp(12));
            paint.setTextAlign(Paint.Align.LEFT);
            canvas.drawText("🔍 অ্যাপ্লিকেশন খুঁজুন...", dp(40), y + dp(24), paint);

            // Apps list
            y += dp(48);
            String[][] pkgs = {
                    {"NilChat", "v1.0.4 • নিরাপদ মেসেঞ্জার", "ইনস্টল"},
                    {"Bangla Editor", "v2.1.0 • বাংলা কোডিং টুল", "ইনস্টল"},
                    {"CyberWeather", "v1.2.0 • আবহাওয়া পূর্বাভাস", "আপডেট"},
                    {"Onuron Maps", "v0.9.5 • অফলাইন মানচিত্র", "ইনস্টল"}
            };

            for (int i = 0; i < pkgs.length; i++) {
                RectF card = new RectF(dp(24), y, w - dp(24), y + dp(62));
                paint.setStyle(Paint.Style.FILL);
                paint.setColor(getSurfaceColor());
                canvas.drawRoundRect(card, dp(14), dp(14), paint);
                paint.setStyle(Paint.Style.STROKE);
                paint.setColor(getBorderColor());
                canvas.drawRoundRect(card, dp(14), dp(14), paint);

                paint.setStyle(Paint.Style.FILL);
                paint.setColor(getTextHighColor());
                paint.setTextSize(dp(15));
                paint.setFakeBoldText(true);
                paint.setTextAlign(Paint.Align.LEFT);
                canvas.drawText(pkgs[i][0] + " ✔", dp(40), y + dp(26), paint);

                paint.setColor(getTextMedColor());
                paint.setTextSize(dp(11.5f));
                paint.setFakeBoldText(false);
                canvas.drawText(toBengaliDigits(pkgs[i][1]), dp(40), y + dp(46), paint);

                // Install/Update button
                RectF instBtn = new RectF(w - dp(96), y + dp(16), w - dp(36), y + dp(46));
                paint.setColor(COLOR_PURPLE);
                canvas.drawRoundRect(instBtn, dp(12), dp(12), paint);
                paint.setColor(0xFFFFFFFF);
                paint.setTextSize(dp(11));
                paint.setTextAlign(Paint.Align.CENTER);
                canvas.drawText(pkgs[i][2], instBtn.centerX(), instBtn.centerY() + dp(4), paint);

                touchAreas.add(new TouchArea(instBtn, "pkg_action_" + i));
                y += dp(70);
            }
        }

        // ─── Screen 18: Android Container (Screen 18) ───────────────────────────────

        private void renderAndroidContainerScreen(Canvas canvas, int w, int h) {
            float y = dp(52);
            paint.setColor(COLOR_CYAN);
            paint.setTextSize(dp(20));
            paint.setFakeBoldText(true);
            paint.setTextAlign(Paint.Align.LEFT);
            canvas.drawText("অ্যান্ড্রয়েড কম্প্যাটিবিলিটি", dp(24), y + dp(18), paint);

            // Container Status Card
            y += dp(36);
            RectF statusCard = new RectF(dp(24), y, w - dp(24), y + dp(70));
            paint.setStyle(Paint.Style.FILL);
            paint.setColor(getSurfaceColor());
            canvas.drawRoundRect(statusCard, dp(16), dp(16), paint);

            paint.setColor(COLOR_GREEN);
            paint.setTextSize(dp(14));
            paint.setFakeBoldText(true);
            paint.setTextAlign(Paint.Align.LEFT);
            canvas.drawText("🟢 কনটেইনার সক্রিয় (LXC Sandbox)", dp(40), y + dp(28), paint);

            paint.setColor(getTextMedColor());
            paint.setTextSize(dp(11.5f));
            paint.setFakeBoldText(false);
            canvas.drawText("আলাদা মেমোরি স্পেস ও সেকম্প আইসোলেশন চালু", dp(40), y + dp(50), paint);

            // Installed apps
            y += dp(86);
            String[] aApps = {"📱 WhatsApp Messenger", "📱 Telegram", "📱 YouTube"};
            for (int i = 0; i < aApps.length; i++) {
                RectF card = new RectF(dp(24), y, w - dp(24), y + dp(54));
                paint.setStyle(Paint.Style.FILL);
                paint.setColor(getSurfaceColor());
                canvas.drawRoundRect(card, dp(14), dp(14), paint);
                paint.setStyle(Paint.Style.STROKE);
                paint.setColor(getBorderColor());
                canvas.drawRoundRect(card, dp(14), dp(14), paint);

                paint.setStyle(Paint.Style.FILL);
                paint.setColor(getTextHighColor());
                paint.setTextSize(dp(14));
                paint.setFakeBoldText(true);
                paint.setTextAlign(Paint.Align.LEFT);
                canvas.drawText(aApps[i], dp(40), y + dp(32), paint);

                touchAreas.add(new TouchArea(card, "android_open_" + i));
                y += dp(62);
            }

            // Install APK Action Pill
            float btnY = h - dp(120);
            RectF apkBtn = new RectF(dp(48), btnY, w - dp(48), btnY + dp(48));
            paint.setStyle(Paint.Style.FILL);
            paint.setColor(COLOR_PURPLE);
            canvas.drawRoundRect(apkBtn, dp(24), dp(24), paint);
            paint.setColor(0xFFFFFFFF);
            paint.setTextSize(dp(15));
            paint.setTextAlign(Paint.Align.CENTER);
            paint.setFakeBoldText(true);
            canvas.drawText("+ APK ইনস্টল করুন", apkBtn.centerX(), btnY + dp(30), paint);
            touchAreas.add(new TouchArea(apkBtn, "install_apk"));
        }

        // ─── Screens 19, 20, 21: OOBE ───────────────────────────────────────────────

        private void renderOobeScreen(Canvas canvas, int w, int h) {
            float y = dp(80);
            paint.setColor(getTextHighColor());
            paint.setTextSize(dp(24));
            paint.setFakeBoldText(true);
            paint.setTextAlign(Paint.Align.CENTER);

            if (screen == Screen.OOBE_1) {
                canvas.drawText("স্বাগতম Onuron OS-এ", w / 2f, y, paint);
                paint.setColor(COLOR_CYAN);
                paint.setTextSize(dp(14));
                paint.setFakeBoldText(false);
                canvas.drawText("স্বাধীন লিনাক্স-ভিত্তিক মোবাইল ওএস", w / 2f, y + dp(30), paint);

                RectF nextBtn = new RectF(dp(48), h - dp(140), w - dp(48), h - dp(92));
                paint.setColor(COLOR_PURPLE);
                canvas.drawRoundRect(nextBtn, dp(24), dp(24), paint);
                paint.setColor(0xFFFFFFFF);
                paint.setTextSize(dp(16));
                paint.setFakeBoldText(true);
                canvas.drawText("শুরু করুন →", nextBtn.centerX(), nextBtn.centerY() + dp(6), paint);
                touchAreas.add(new TouchArea(nextBtn, "oobe_next_2"));

            } else if (screen == Screen.OOBE_2) {
                canvas.drawText("ভাষা নির্বাচন করুন", w / 2f, y, paint);

                float ly = y + dp(50);
                String[] langs = {"বাংলা (বাংলাদেশ)", "English (United States)", "हिन्दी"};
                for (int i = 0; i < langs.length; i++) {
                    RectF card = new RectF(dp(36), ly, w - dp(36), ly + dp(50));
                    paint.setStyle(Paint.Style.FILL);
                    paint.setColor(i == 0 ? COLOR_PURPLE : getSurfaceColor());
                    canvas.drawRoundRect(card, dp(14), dp(14), paint);
                    paint.setColor(i == 0 ? 0xFFFFFFFF : getTextHighColor());
                    paint.setTextSize(dp(14));
                    canvas.drawText(langs[i], card.centerX(), ly + dp(31), paint);
                    touchAreas.add(new TouchArea(card, "oobe_lang_" + i));
                    ly += dp(62);
                }

                RectF nextBtn = new RectF(dp(48), h - dp(140), w - dp(48), h - dp(92));
                paint.setColor(COLOR_PURPLE);
                canvas.drawRoundRect(nextBtn, dp(24), dp(24), paint);
                paint.setColor(0xFFFFFFFF);
                paint.setTextSize(dp(16));
                paint.setFakeBoldText(true);
                canvas.drawText("পরবর্তী ধাপ →", nextBtn.centerX(), nextBtn.centerY() + dp(6), paint);
                touchAreas.add(new TouchArea(nextBtn, "oobe_next_3"));

            } else {
                canvas.drawText("নিরাপত্তা ও পিন সেটআপ", w / 2f, y, paint);
                paint.setColor(getTextMedColor());
                paint.setTextSize(dp(14));
                paint.setFakeBoldText(false);
                canvas.drawText("ডিভাইসের জন্য ৪-সংখ্যার পিন দিন", w / 2f, y + dp(30), paint);

                paint.setColor(COLOR_CYAN);
                paint.setTextSize(dp(28));
                canvas.drawText("••••", w / 2f, y + dp(80), paint);

                RectF finishBtn = new RectF(dp(48), h - dp(140), w - dp(48), h - dp(92));
                paint.setColor(COLOR_GREEN);
                canvas.drawRoundRect(finishBtn, dp(24), dp(24), paint);
                paint.setColor(0xFFFFFFFF);
                paint.setTextSize(dp(16));
                paint.setFakeBoldText(true);
                canvas.drawText("সমাপ্ত করুন ✔", finishBtn.centerX(), finishBtn.centerY() + dp(6), paint);
                touchAreas.add(new TouchArea(finishBtn, "oobe_finish"));
            }
        }

        // ─── Screen 22: Settings Hub (Screen 22) ────────────────────────────────────

        private void renderSettingsScreen(Canvas canvas, int w, int h) {
            float y = dp(52);
            paint.setColor(COLOR_PURPLE);
            paint.setTextSize(dp(20));
            paint.setFakeBoldText(true);
            paint.setTextAlign(Paint.Align.LEFT);
            canvas.drawText("সেটিংস", dp(24), y + dp(18), paint);

            // Search pill
            y += dp(32);
            RectF searchPill = new RectF(dp(24), y, w - dp(24), y + dp(38));
            paint.setStyle(Paint.Style.FILL);
            paint.setColor(getSurfaceColor());
            canvas.drawRoundRect(searchPill, dp(19), dp(19), paint);

            paint.setColor(getTextMedColor());
            paint.setTextSize(dp(12));
            paint.setTextAlign(Paint.Align.LEFT);
            canvas.drawText("🔍 সেটিংস খুঁজুন...", dp(40), y + dp(24), paint);

            // 10 Navigation Rows
            y += dp(48);
            String[][] setRows = {
                    {"📶 সংযোগ ও নেটওয়ার্ক", "ওয়াই-ফাই, ব্লুটুথ, সিম কার্ড", "nav_set_wifi"},
                    {"ᛒ ব্লুটুথ ডিভাইস", "হেডফোন, স্মার্টওয়াচ", "nav_set_bt"},
                    {"☀️ ডিসপ্লে ও উজ্জ্বলতা", "ডার্ক মোড, ফন্ট সাইজ", "nav_set_display"},
                    {"🔊 শব্দ ও কম্পন", "ভলিউম, রিংটোন", "nav_set_sound"},
                    {"🔒 নিরাপত্তা ও লক", "পিন লক, বায়োমেট্রিক", "nav_set_security"},
                    {"🔋 ব্যাটারি ও পারফরম্যান্স", "৮৫%, ব্যাকআপ মোড", "nav_set_battery"},
                    {"💾 স্টোরেজ ও মেমরি", "১২৮ GB এর মধ্যে ৪২ GB ব্যবহৃত", "nav_set_storage"},
                    {"🌐 ভাষা ও ইনপুট", "বাংলা (বাংলাদেশ)", "nav_set_language"},
                    {"♿ সহজলভ্যতা (A11y)", "স্ক্রিন রিডার, উচ্চ বৈসাদৃশ্য", "nav_set_a11y"},
                    {"🛡️ ভিপিএন ও প্রক্সি", "WireGuard, অন-ডিমান্ড", "nav_set_vpn"},
                    {"☁️ ব্যাকআপ ও রিস্টোর", "ক্লাউড সিঙ্ক, স্থানীয় ফাইল", "nav_set_backup"},
                    {"🔄 সিস্টেম আপডেট", "Onuron OS v১.২ প্রস্তুত", "nav_set_update"},
                    {"ℹ️ ডিভাইস পরিচিতি", "Samsung Galaxy S25", "nav_set_about"}
            };

            for (int i = 0; i < setRows.length; i++) {
                if (y > h - dp(70)) break; // Prevent bottom nav overlap
                RectF card = new RectF(dp(24), y, w - dp(24), y + dp(50));
                paint.setStyle(Paint.Style.FILL);
                paint.setColor(getSurfaceColor());
                canvas.drawRoundRect(card, dp(12), dp(12), paint);
                paint.setStyle(Paint.Style.STROKE);
                paint.setColor(getBorderColor());
                canvas.drawRoundRect(card, dp(12), dp(12), paint);

                paint.setStyle(Paint.Style.FILL);
                paint.setColor(getTextHighColor());
                paint.setTextSize(dp(13.5f));
                paint.setFakeBoldText(true);
                paint.setTextAlign(Paint.Align.LEFT);
                canvas.drawText(setRows[i][0], dp(38), y + dp(22), paint);

                paint.setColor(getTextMedColor());
                paint.setTextSize(dp(10.5f));
                paint.setFakeBoldText(false);
                canvas.drawText(setRows[i][1], dp(38), y + dp(38), paint);

                // Right chevron >
                paint.setColor(getTextDimColor());
                paint.setTextSize(dp(14));
                paint.setTextAlign(Paint.Align.RIGHT);
                canvas.drawText("〉", w - dp(36), y + dp(30), paint);

                touchAreas.add(new TouchArea(card, setRows[i][2]));
                y += dp(56);
            }
        }

        // ─── Settings Sub-screens (Screens 23 to 35) ────────────────────────────────

        private void renderSettingsWifi(Canvas canvas, int w, int h) {
            float y = renderSubHeader(canvas, w, "ওয়াই-ফাই সেটিংস");

            // Main Switch Card
            RectF swCard = new RectF(dp(24), y, w - dp(24), y + dp(56));
            drawToggleCard(canvas, swCard, "ওয়াই-ফাই সক্রিয়", wifiEnabled, "sub_toggle_wifi");
            y += dp(68);

            // Available networks
            String[][] nets = {
                    {"Onuron_5G_Fiber", "সংযুক্ত • সিগন্যাল চমৎকার", "1"},
                    {"NilHome_Mesh", "সংরক্ষিত", "0"},
                    {"Qualcomm_DevLab", "উন্মুক্ত", "0"}
            };
            for (String[] net : nets) {
                RectF card = new RectF(dp(24), y, w - dp(24), y + dp(54));
                paint.setStyle(Paint.Style.FILL);
                paint.setColor(getSurfaceColor());
                canvas.drawRoundRect(card, dp(12), dp(12), paint);

                paint.setColor("1".equals(net[2]) ? COLOR_PURPLE : getTextHighColor());
                paint.setTextSize(dp(14));
                paint.setFakeBoldText(true);
                paint.setTextAlign(Paint.Align.LEFT);
                canvas.drawText("📶 " + net[0], dp(38), y + dp(25), paint);

                paint.setColor(getTextMedColor());
                paint.setTextSize(dp(11));
                paint.setFakeBoldText(false);
                canvas.drawText(net[1], dp(38), y + dp(43), paint);

                y += dp(60);
            }
        }

        private void renderSettingsBluetooth(Canvas canvas, int w, int h) {
            float y = renderSubHeader(canvas, w, "ব্লুটুথ ডিভাইস");
            RectF swCard = new RectF(dp(24), y, w - dp(24), y + dp(56));
            drawToggleCard(canvas, swCard, "ব্লুটুথ সক্রিয়", btEnabled, "sub_toggle_bt");
            y += dp(68);

            String[] devs = {"🎧 NilBuds Pro (সংযুক্ত)", "⌚ Galaxy Watch 7", "💻 MacBook Air"};
            for (String dev : devs) {
                RectF card = new RectF(dp(24), y, w - dp(24), y + dp(50));
                paint.setStyle(Paint.Style.FILL);
                paint.setColor(getSurfaceColor());
                canvas.drawRoundRect(card, dp(12), dp(12), paint);

                paint.setColor(getTextHighColor());
                paint.setTextSize(dp(14));
                paint.setFakeBoldText(true);
                paint.setTextAlign(Paint.Align.LEFT);
                canvas.drawText(dev, dp(38), y + dp(31), paint);
                y += dp(58);
            }
        }

        private void renderSettingsDisplay(Canvas canvas, int w, int h) {
            float y = renderSubHeader(canvas, w, "ডিসপ্লে ও উজ্জ্বলতা");

            // Theme Selection Cards: [ ডার্ক মোড ]  [ লাইট মোড ]
            float tw = (w - dp(48 + 12)) / 2f;
            RectF darkCard = new RectF(dp(24), y, dp(24) + tw, y + dp(80));
            paint.setStyle(Paint.Style.FILL);
            paint.setColor(darkMode ? COLOR_PURPLE : getSurfaceColor());
            canvas.drawRoundRect(darkCard, dp(16), dp(16), paint);
            paint.setColor(darkMode ? 0xFFFFFFFF : getTextHighColor());
            paint.setTextSize(dp(14));
            paint.setFakeBoldText(true);
            paint.setTextAlign(Paint.Align.CENTER);
            canvas.drawText("🌙 ডার্ক মোড", darkCard.centerX(), y + dp(45), paint);
            touchAreas.add(new TouchArea(darkCard, "theme_dark"));

            RectF lightCard = new RectF(dp(36) + tw, y, w - dp(24), y + dp(80));
            paint.setColor(!darkMode ? COLOR_PURPLE : getSurfaceColor());
            canvas.drawRoundRect(lightCard, dp(16), dp(16), paint);
            paint.setColor(!darkMode ? 0xFFFFFFFF : getTextHighColor());
            canvas.drawText("☀️ লাইট মোড", lightCard.centerX(), y + dp(45), paint);
            touchAreas.add(new TouchArea(lightCard, "theme_light"));

            y += dp(96);
            // Brightness Card
            RectF brCard = new RectF(dp(24), y, w - dp(24), y + dp(56));
            paint.setColor(getSurfaceColor());
            canvas.drawRoundRect(brCard, dp(14), dp(14), paint);
            paint.setColor(getTextHighColor());
            paint.setTextSize(dp(13.5f));
            paint.setTextAlign(Paint.Align.LEFT);
            canvas.drawText("উজ্জ্বলতা স্তর: " + toBengaliDigits(String.valueOf(brightnessLevel)) + "%", dp(40), y + dp(33), paint);
            touchAreas.add(new TouchArea(brCard, "cycle_brightness"));
        }

        private void renderSettingsSound(Canvas canvas, int w, int h) {
            float y = renderSubHeader(canvas, w, "শব্দ ও কম্পন");

            String[] volTypes = {"মিডিয়া ভলিউম", "রিংটোন", "অ্যালার্ম"};
            for (String vt : volTypes) {
                RectF card = new RectF(dp(24), y, w - dp(24), y + dp(54));
                paint.setStyle(Paint.Style.FILL);
                paint.setColor(getSurfaceColor());
                canvas.drawRoundRect(card, dp(12), dp(12), paint);

                paint.setColor(getTextHighColor());
                paint.setTextSize(dp(13.5f));
                paint.setFakeBoldText(true);
                paint.setTextAlign(Paint.Align.LEFT);
                canvas.drawText("🔊 " + vt, dp(38), y + dp(32), paint);

                paint.setColor(COLOR_PURPLE);
                paint.setTextAlign(Paint.Align.RIGHT);
                canvas.drawText(toBengaliDigits(String.valueOf(soundVolume)) + "%", w - dp(38), y + dp(32), paint);
                touchAreas.add(new TouchArea(card, "cycle_volume"));
                y += dp(62);
            }
        }

        private void renderSettingsSecurity(Canvas canvas, int w, int h) {
            float y = renderSubHeader(canvas, w, "নিরাপত্তা ও লক");
            RectF swCard = new RectF(dp(24), y, w - dp(24), y + dp(56));
            drawToggleCard(canvas, swCard, "বায়োমেট্রিক ফিঙ্গারপ্রিন্ট", biometricEnabled, "sub_toggle_biometric");
            y += dp(68);

            RectF pinCard = new RectF(dp(24), y, w - dp(24), y + dp(56));
            paint.setStyle(Paint.Style.FILL);
            paint.setColor(getSurfaceColor());
            canvas.drawRoundRect(pinCard, dp(12), dp(12), paint);
            paint.setColor(getTextHighColor());
            paint.setTextSize(dp(14));
            paint.setFakeBoldText(true);
            paint.setTextAlign(Paint.Align.LEFT);
            canvas.drawText("🔒 স্ক্রিন লক পিন: সেট করা আছে", dp(38), y + dp(33), paint);
        }

        private void renderSettingsBattery(Canvas canvas, int w, int h) {
            float y = renderSubHeader(canvas, w, "ব্যাটারি ও পারফরম্যান্স");

            // Battery Level Display
            RectF batCard = new RectF(dp(24), y, w - dp(24), y + dp(84));
            paint.setStyle(Paint.Style.FILL);
            paint.setColor(getSurfaceColor());
            canvas.drawRoundRect(batCard, dp(16), dp(16), paint);

            paint.setColor(COLOR_GREEN);
            paint.setTextSize(dp(32));
            paint.setFakeBoldText(true);
            paint.setTextAlign(Paint.Align.LEFT);
            canvas.drawText(toBengaliDigits(String.valueOf(batteryLevel)) + "%", dp(40), y + dp(46), paint);

            paint.setColor(getTextMedColor());
            paint.setTextSize(dp(12));
            paint.setFakeBoldText(false);
            canvas.drawText(isCharging ? "চার্জ হচ্ছে..." : "বাকি প্রায় ১৮ ঘণ্টা ২০ মিনিট", dp(40), y + dp(68), paint);

            y += dp(96);
            RectF swCard = new RectF(dp(24), y, w - dp(24), y + dp(56));
            drawToggleCard(canvas, swCard, "পাওয়ার সেভিং মোড", batterySaver, "sub_toggle_saver");
        }

        private void renderSettingsStorage(Canvas canvas, int w, int h) {
            float y = renderSubHeader(canvas, w, "স্টোরেজ ও মেমরি");

            RectF sCard = new RectF(dp(24), y, w - dp(24), y + dp(90));
            paint.setStyle(Paint.Style.FILL);
            paint.setColor(getSurfaceColor());
            canvas.drawRoundRect(sCard, dp(16), dp(16), paint);

            paint.setColor(COLOR_PURPLE);
            paint.setTextSize(dp(24));
            paint.setFakeBoldText(true);
            paint.setTextAlign(Paint.Align.LEFT);
            canvas.drawText("৪২ GB ব্যবহৃত", dp(40), y + dp(38), paint);

            paint.setColor(getTextMedColor());
            paint.setTextSize(dp(12));
            paint.setFakeBoldText(false);
            canvas.drawText("মোট স্টোরেজ: ১২৮ GB (Snapdragon UFS 4.0)", dp(40), y + dp(64), paint);

            y += dp(104);
            RectF cleanBtn = new RectF(dp(36), y, w - dp(36), y + dp(48));
            paint.setColor(COLOR_PURPLE);
            canvas.drawRoundRect(cleanBtn, dp(24), dp(24), paint);
            paint.setColor(0xFFFFFFFF);
            paint.setTextSize(dp(15));
            paint.setFakeBoldText(true);
            paint.setTextAlign(Paint.Align.CENTER);
            canvas.drawText("জাঙ্ক ফাইল পরিষ্কার করুন (১.২ GB)", cleanBtn.centerX(), y + dp(30), paint);
            touchAreas.add(new TouchArea(cleanBtn, "sub_clean_storage"));
        }

        private void renderSettingsLanguage(Canvas canvas, int w, int h) {
            float y = renderSubHeader(canvas, w, "ভাষা ও ইনপুট");

            String[] langs = {"বাংলা (বাংলাদেশ) ✔", "English (United States)", "हिन्दी"};
            for (int i = 0; i < langs.length; i++) {
                RectF card = new RectF(dp(24), y, w - dp(24), y + dp(50));
                paint.setStyle(Paint.Style.FILL);
                paint.setColor(i == 0 ? COLOR_PURPLE : getSurfaceColor());
                canvas.drawRoundRect(card, dp(12), dp(12), paint);
                paint.setColor(i == 0 ? 0xFFFFFFFF : getTextHighColor());
                paint.setTextSize(dp(14));
                paint.setTextAlign(Paint.Align.LEFT);
                canvas.drawText(langs[i], dp(40), y + dp(31), paint);
                y += dp(58);
            }
        }

        private void renderSettingsAccessibility(Canvas canvas, int w, int h) {
            float y = renderSubHeader(canvas, w, "সহজলভ্যতা (Accessibility)");
            RectF sw1 = new RectF(dp(24), y, w - dp(24), y + dp(56));
            drawToggleCard(canvas, sw1, "উচ্চ বৈসাদৃশ্য টেক্সট", false, "sub_a11y_1");
            y += dp(66);

            RectF sw2 = new RectF(dp(24), y, w - dp(24), y + dp(56));
            drawToggleCard(canvas, sw2, "স্ক্রিন রিডার (TalkBack)", false, "sub_a11y_2");
        }

        private void renderSettingsVpn(Canvas canvas, int w, int h) {
            float y = renderSubHeader(canvas, w, "ভিপিএন ও প্রক্সি");
            RectF swCard = new RectF(dp(24), y, w - dp(24), y + dp(56));
            drawToggleCard(canvas, swCard, "WireGuard VPN", vpnConnected, "sub_toggle_vpn");
            y += dp(68);

            RectF vpnCard = new RectF(dp(24), y, w - dp(24), y + dp(54));
            paint.setStyle(Paint.Style.FILL);
            paint.setColor(getSurfaceColor());
            canvas.drawRoundRect(vpnCard, dp(12), dp(12), paint);
            paint.setColor(getTextHighColor());
            paint.setTextSize(dp(14));
            paint.setFakeBoldText(true);
            paint.setTextAlign(Paint.Align.LEFT);
            canvas.drawText("🛡️ Onuron-Secure-Node-1", dp(40), y + dp(32), paint);
        }

        private void renderSettingsBackup(Canvas canvas, int w, int h) {
            float y = renderSubHeader(canvas, w, "ব্যাকআপ ও রিস্টোর");

            RectF bCard = new RectF(dp(24), y, w - dp(24), y + dp(84));
            paint.setStyle(Paint.Style.FILL);
            paint.setColor(getSurfaceColor());
            canvas.drawRoundRect(bCard, dp(16), dp(16), paint);

            paint.setColor(COLOR_CYAN);
            paint.setTextSize(dp(16));
            paint.setFakeBoldText(true);
            paint.setTextAlign(Paint.Align.LEFT);
            canvas.drawText("ক্লাউড ব্যাকআপ সক্রিয়", dp(40), y + dp(34), paint);

            paint.setColor(getTextMedColor());
            paint.setTextSize(dp(12));
            paint.setFakeBoldText(false);
            canvas.drawText("সর্বশেষ ব্যাকআপ: আজ দুপুর ১২:৩০", dp(40), y + dp(58), paint);

            y += dp(96);
            RectF bBtn = new RectF(dp(36), y, w - dp(36), y + dp(48));
            paint.setColor(COLOR_PURPLE);
            canvas.drawRoundRect(bBtn, dp(24), dp(24), paint);
            paint.setColor(0xFFFFFFFF);
            paint.setTextSize(dp(15));
            paint.setTextAlign(Paint.Align.CENTER);
            paint.setFakeBoldText(true);
            canvas.drawText("এখন ব্যাকআপ নিন", bBtn.centerX(), y + dp(30), paint);
            touchAreas.add(new TouchArea(bBtn, "sub_backup_now"));
        }

        private void renderSettingsUpdate(Canvas canvas, int w, int h) {
            float y = renderSubHeader(canvas, w, "সিস্টেম আপডেট");

            RectF uCard = new RectF(dp(24), y, w - dp(24), y + dp(100));
            paint.setStyle(Paint.Style.FILL);
            paint.setColor(getSurfaceColor());
            canvas.drawRoundRect(uCard, dp(16), dp(16), paint);

            paint.setColor(COLOR_GREEN);
            paint.setTextSize(dp(18));
            paint.setFakeBoldText(true);
            paint.setTextAlign(Paint.Align.LEFT);
            canvas.drawText("Onuron OS v১.২ প্রস্তুত", dp(40), y + dp(38), paint);

            paint.setColor(getTextMedColor());
            paint.setTextSize(dp(12));
            paint.setFakeBoldText(false);
            canvas.drawText("নতুন কার্নেল প্যাচ, ফন্ট পারফরম্যান্স ও আধুনিক ইউআই", dp(40), y + dp(64), paint);

            y += dp(112);
            RectF uBtn = new RectF(dp(36), y, w - dp(36), y + dp(48));
            paint.setColor(COLOR_PURPLE);
            canvas.drawRoundRect(uBtn, dp(24), dp(24), paint);
            paint.setColor(0xFFFFFFFF);
            paint.setTextSize(dp(15));
            paint.setTextAlign(Paint.Align.CENTER);
            paint.setFakeBoldText(true);
            canvas.drawText("আপডেট ইনস্টল করুন", uBtn.centerX(), y + dp(30), paint);
            touchAreas.add(new TouchArea(uBtn, "sub_install_update"));
        }

        private void renderAboutScreen(Canvas canvas, int w, int h) {
            float y = renderSubHeader(canvas, w, "ডিভাইস পরিচিতি");

            String[][] aboutData = {
                    {"ডিভাইসের নাম", "Samsung Galaxy S25 (NilDevice)"},
                    {"অপারেটিং সিস্টেম", "Onuron OS 1.0 (Alap Engine)"},
                    {"কার্নেল ভার্সন", "Linux LTS 6.6.21-onuron-s25"},
                    {"PID 1 Init", "nilinit supervision layer"},
                    {"হার্ডওয়্যার", "Snapdragon 8 Elite • 12GB LPDDR5X"},
                    {"বিল্ড স্ট্যাটাস", "Production Native Verified ✔"}
            };

            for (String[] ad : aboutData) {
                RectF card = new RectF(dp(24), y, w - dp(24), y + dp(48));
                paint.setStyle(Paint.Style.FILL);
                paint.setColor(getSurfaceColor());
                canvas.drawRoundRect(card, dp(12), dp(12), paint);
                paint.setStyle(Paint.Style.STROKE);
                paint.setColor(getBorderColor());
                canvas.drawRoundRect(card, dp(12), dp(12), paint);

                paint.setStyle(Paint.Style.FILL);
                paint.setColor(COLOR_PURPLE);
                paint.setTextSize(dp(12));
                paint.setFakeBoldText(true);
                paint.setTextAlign(Paint.Align.LEFT);
                canvas.drawText(ad[0], dp(38), y + dp(20), paint);

                paint.setColor(getTextHighColor());
                paint.setTextSize(dp(13));
                paint.setFakeBoldText(false);
                canvas.drawText(ad[1], dp(38), y + dp(38), paint);

                y += dp(54);
            }
        }

        // Sub-screen header helper
        private float renderSubHeader(Canvas canvas, int w, String title) {
            float y = dp(52);
            RectF backBtn = new RectF(dp(20), y, dp(54), y + dp(34));
            paint.setStyle(Paint.Style.FILL);
            paint.setColor(getSurfaceColor());
            canvas.drawRoundRect(backBtn, dp(12), dp(12), paint);
            paint.setColor(getTextHighColor());
            paint.setTextSize(dp(18));
            paint.setFakeBoldText(true);
            paint.setTextAlign(Paint.Align.CENTER);
            canvas.drawText("〈", backBtn.centerX(), y + dp(24), paint);
            touchAreas.add(new TouchArea(backBtn, "sub_back"));

            paint.setColor(COLOR_PURPLE);
            paint.setTextSize(dp(18));
            paint.setTextAlign(Paint.Align.LEFT);
            canvas.drawText(title, dp(66), y + dp(24), paint);

            return y + dp(48);
        }

        // Toggle card helper
        private void drawToggleCard(Canvas canvas, RectF card, String label, boolean active, String id) {
            paint.setStyle(Paint.Style.FILL);
            paint.setColor(getSurfaceColor());
            canvas.drawRoundRect(card, dp(14), dp(14), paint);
            paint.setStyle(Paint.Style.STROKE);
            paint.setColor(getBorderColor());
            canvas.drawRoundRect(card, dp(14), dp(14), paint);

            paint.setStyle(Paint.Style.FILL);
            paint.setColor(getTextHighColor());
            paint.setTextSize(dp(14));
            paint.setFakeBoldText(true);
            paint.setTextAlign(Paint.Align.LEFT);
            canvas.drawText(label, card.left + dp(16), card.centerY() + dp(5), paint);

            // Switch toggle graphics
            float swW = dp(42);
            float swH = dp(24);
            float swX = card.right - dp(16) - swW;
            float swY = card.centerY() - swH / 2f;
            paint.setColor(active ? COLOR_PURPLE : (darkMode ? 0xFF3B2E56 : 0xFFCBD5E1));
            canvas.drawRoundRect(swX, swY, swX + swW, swY + swH, swH / 2f, swH / 2f, paint);

            paint.setColor(0xFFFFFFFF);
            float knobX = active ? (swX + swW - dp(12)) : (swX + dp(12));
            canvas.drawCircle(knobX, swY + swH / 2f, dp(9), paint);

            touchAreas.add(new TouchArea(card, id));
        }

        // ─── Touch Events Handling ──────────────────────────────────────────────────

        @Override
        public boolean onTouchEvent(MotionEvent event) {
            if (event.getAction() == MotionEvent.ACTION_DOWN) {
                float x = event.getX();
                float y = event.getY();

                for (TouchArea area : touchAreas) {
                    if (area.bounds.contains(x, y)) {
                        vibrate();
                        handleAction(area.id);
                        invalidate();
                        return true;
                    }
                }
            }
            return super.onTouchEvent(event);
        }

        private void handleAction(String id) {
            switch (id) {
                case "nav_home":
                    prevScreen = screen;
                    screen = Screen.HOME;
                    activity.setTerminalInputVisible(false);
                    activity.setMessageInputVisible(false);
                    activity.hideBrowser();
                    inActiveChat = false;
                    break;
                case "nav_back":
                case "sub_back":
                    handleBack();
                    break;
                case "nav_recents":
                    prevScreen = screen;
                    screen = Screen.RECENTS;
                    break;
                case "open_control_center":
                    prevScreen = screen;
                    screen = Screen.CONTROL_CENTER;
                    break;
                case "close_control":
                    screen = prevScreen;
                    break;
                case "unlock_device":
                    screen = Screen.HOME;
                    break;
                case "lock_emergency":
                    screen = Screen.PHONE;
                    break;
                case "lock_camera":
                    screen = Screen.CAMERA;
                    break;
                case "app_phone":
                    screen = Screen.PHONE;
                    break;
                case "app_browser":
                    activity.showBrowser("https://onuron.org");
                    break;
                case "app_messages":
                    screen = Screen.MESSAGES;
                    break;
                case "app_files":
                    screen = Screen.FILES;
                    break;
                case "app_camera":
                    screen = Screen.CAMERA;
                    break;
                case "app_notes":
                    screen = Screen.NOTES;
                    break;
                case "app_music":
                    screen = Screen.MUSIC;
                    break;
                case "app_calculator":
                    screen = Screen.CALCULATOR;
                    break;
                case "app_settings":
                    screen = Screen.SETTINGS;
                    break;
                case "app_terminal":
                    screen = Screen.TERMINAL;
                    activity.setTerminalInputVisible(true);
                    break;
                case "app_softbus":
                    screen = Screen.SOFTBUS;
                    break;
                case "app_nilpkg":
                    screen = Screen.NILPKG;
                    break;
                case "theme_dark":
                    darkMode = true;
                    break;
                case "theme_light":
                    darkMode = false;
                    break;
                case "toggle_dark_theme":
                    darkMode = !darkMode;
                    break;
                case "toggle_wifi":
                case "sub_toggle_wifi":
                    wifiEnabled = !wifiEnabled;
                    break;
                case "toggle_bt":
                case "sub_toggle_bt":
                    btEnabled = !btEnabled;
                    break;
                case "toggle_torch":
                    torchEnabled = !torchEnabled;
                    break;
                case "toggle_silent":
                    soundVolume = soundVolume == 0 ? 75 : 0;
                    break;
                case "toggle_airplane":
                    airplaneMode = !airplaneMode;
                    break;
                case "toggle_cellular":
                    cellularEnabled = !cellularEnabled;
                    break;
                case "sub_toggle_saver":
                    batterySaver = !batterySaver;
                    break;
                case "cycle_brightness":
                    brightnessLevel = (brightnessLevel >= 100) ? 25 : (brightnessLevel + 25);
                    break;
                case "cycle_volume":
                    soundVolume = (soundVolume >= 100) ? 0 : (soundVolume + 25);
                    break;
                case "clear_notifications":
                    Toast.makeText(getContext(), "সকল বিজ্ঞপ্তি মোছা হয়েছে", Toast.LENGTH_SHORT).show();
                    break;
                case "clear_all_recents":
                    recentsList.clear();
                    screen = Screen.HOME;
                    break;
                case "act_call_screen":
                    inCallSec = 1;
                    screen = Screen.IN_CALL;
                    break;
                case "incall_end":
                    screen = Screen.PHONE;
                    break;
                case "incall_mute":
                    inCallMuted = !inCallMuted;
                    break;
                case "incall_speaker":
                    inCallSpeaker = !inCallSpeaker;
                    break;
                case "act_del":
                    if (!dialNumber.isEmpty()) {
                        dialNumber = dialNumber.substring(0, dialNumber.length() - 1);
                    }
                    break;
                case "phone_tab_0": phoneTab = 0; break;
                case "phone_tab_1": phoneTab = 1; break;
                case "phone_tab_2": phoneTab = 2; break;
                case "camera_toggle_flash":
                    cameraFlash = !cameraFlash;
                    break;
                case "camera_shutter":
                    shutterFlashAnim = 2;
                    vibrate();
                    Toast.makeText(getContext(), "ছবি সংরক্ষিত হয়েছে 📸", Toast.LENGTH_SHORT).show();
                    break;
                case "camera_mode_0": cameraMode = 0; break;
                case "camera_mode_1": cameraMode = 1; break;
                case "notes_add_new":
                    notesList.add(0, new String[]{"নতুন নোট " + (notesList.size() + 1), "অনুরণ ওএস দ্রুত টাস্ক নোট।", "ব্যক্তিগত", "আজ"});
                    Toast.makeText(getContext(), "নতুন নোট তৈরি হয়েছে", Toast.LENGTH_SHORT).show();
                    break;
                case "msg_new_chat":
                    activeRecipient = "নতুন প্রেরক";
                    inActiveChat = true;
                    activity.setMessageInputVisible(true);
                    break;
                case "chat_back":
                    inActiveChat = false;
                    activity.setMessageInputVisible(false);
                    break;
                case "music_toggle_play":
                    musicPlaying = !musicPlaying;
                    break;
                case "music_next":
                    musicCurrentTrack = (musicCurrentTrack + 1) % musicTracks.size();
                    break;
                case "music_prev":
                    musicCurrentTrack = (musicCurrentTrack == 0) ? musicTracks.size() - 1 : musicCurrentTrack - 1;
                    break;
                // Settings navigation
                case "nav_set_wifi": screen = Screen.SETTINGS_WIFI; break;
                case "nav_set_bt": screen = Screen.SETTINGS_BLUETOOTH; break;
                case "nav_set_display": screen = Screen.SETTINGS_DISPLAY; break;
                case "nav_set_sound": screen = Screen.SETTINGS_SOUND; break;
                case "nav_set_security": screen = Screen.SETTINGS_SECURITY; break;
                case "nav_set_battery": screen = Screen.SETTINGS_BATTERY; break;
                case "nav_set_storage": screen = Screen.SETTINGS_STORAGE; break;
                case "nav_set_language": screen = Screen.SETTINGS_LANGUAGE; break;
                case "nav_set_a11y": screen = Screen.SETTINGS_ACCESSIBILITY; break;
                case "nav_set_vpn": screen = Screen.SETTINGS_VPN; break;
                case "nav_set_backup": screen = Screen.SETTINGS_BACKUP; break;
                case "nav_set_update": screen = Screen.SETTINGS_UPDATE; break;
                case "nav_set_about": screen = Screen.ABOUT; break;
                default:
                    if (id.startsWith("key_")) {
                        dialNumber += id.substring(4);
                    } else if (id.startsWith("notes_filter_")) {
                        notesCategory = Integer.parseInt(id.substring(13));
                    } else if (id.startsWith("camera_zoom_")) {
                        cameraZoom = Integer.parseInt(id.substring(12));
                    } else if (id.startsWith("open_chat_")) {
                        int idx = Integer.parseInt(id.substring(10));
                        if (idx < messageThreads.size()) {
                            activeRecipient = messageThreads.get(idx)[0];
                            inActiveChat = true;
                            activity.setMessageInputVisible(true);
                        }
                    } else if (id.startsWith("calc_btn_")) {
                        handleCalcButton(id.substring(9));
                    } else if (id.startsWith("calc_sci_")) {
                        handleCalcSci(id.substring(9));
                    }
                    break;
            }
        }

        private void handleCalcButton(String btn) {
            if ("C".equals(btn)) {
                calcRawInput = "0";
                calcFormula = "";
                calcDisplay = "০";
            } else if ("=".equals(btn)) {
                try {
                    calcFormula = toBengaliDigits(calcRawInput) + " =";
                    double res = evalMath(calcRawInput);
                    if (res == (long) res) {
                        calcDisplay = toBengaliDigits(String.valueOf((long) res));
                    } else {
                        calcDisplay = toBengaliDigits(String.format(Locale.getDefault(), "%.2f", res));
                    }
                    calcRawInput = String.valueOf(res);
                } catch (Exception e) {
                    calcDisplay = "ত্রুটি";
                }
            } else if ("±".equals(btn)) {
                if (calcRawInput.startsWith("-")) {
                    calcRawInput = calcRawInput.substring(1);
                } else {
                    calcRawInput = "-" + calcRawInput;
                }
                calcDisplay = toBengaliDigits(calcRawInput);
            } else if ("()".equals(btn)) {
                calcRawInput += "(";
                calcDisplay = toBengaliDigits(calcRawInput);
            } else {
                if ("0".equals(calcRawInput) && !".".equals(btn) && !"+-×÷%".contains(btn)) {
                    calcRawInput = btn;
                } else {
                    calcRawInput += btn;
                }
                calcDisplay = toBengaliDigits(calcRawInput);
            }
        }

        private void handleCalcSci(String sci) {
            try {
                double val = Double.parseDouble(calcRawInput);
                double res = 0;
                if ("sin".equals(sci)) res = Math.sin(Math.toRadians(val));
                else if ("cos".equals(sci)) res = Math.cos(Math.toRadians(val));
                else if ("√".equals(sci)) res = Math.sqrt(val);
                else if ("π".equals(sci)) res = Math.PI;

                calcFormula = sci + "(" + toBengaliDigits(calcRawInput) + ") =";
                calcDisplay = toBengaliDigits(String.format(Locale.getDefault(), "%.3f", res));
                calcRawInput = String.valueOf(res);
            } catch (Exception e) {
                calcDisplay = "ত্রুটি";
            }
        }

        private double evalMath(String str) {
            String sanitized = str.replace("×", "*").replace("÷", "/").replace("−", "-");
            // Simple expression evaluator
            return new Object() {
                int pos = -1, ch;
                void nextChar() { ch = (++pos < sanitized.length()) ? sanitized.charAt(pos) : -1; }
                boolean eat(int charToEat) {
                    while (ch == ' ') nextChar();
                    if (ch == charToEat) { nextChar(); return true; }
                    return false;
                }
                double parse() {
                    nextChar();
                    double x = parseExpression();
                    if (pos < sanitized.length()) throw new RuntimeException("Unexpected: " + (char)ch);
                    return x;
                }
                double parseExpression() {
                    double x = parseTerm();
                    for (;;) {
                        if (eat('+')) x += parseTerm();
                        else if (eat('-')) x -= parseTerm();
                        else return x;
                    }
                }
                double parseTerm() {
                    double x = parseFactor();
                    for (;;) {
                        if (eat('*')) x *= parseFactor();
                        else if (eat('/')) x /= parseFactor();
                        else if (eat('%')) x %= parseFactor();
                        else return x;
                    }
                }
                double parseFactor() {
                    if (eat('+')) return parseFactor();
                    if (eat('-')) return -parseFactor();
                    double x;
                    int startPos = this.pos;
                    if (eat('(')) {
                        x = parseExpression();
                        eat(')');
                    } else if ((ch >= '0' && ch <= '9') || ch == '.') {
                        while ((ch >= '0' && ch <= '9') || ch == '.') nextChar();
                        x = Double.parseDouble(sanitized.substring(startPos, this.pos));
                    } else {
                        throw new RuntimeException("Unexpected: " + (char)ch);
                    }
                    return x;
                }
            }.parse();
        }

        public void runCommand(String cmd) {
            termLines.add("> " + cmd);
            String c = cmd.trim();
            if ("clear".equalsIgnoreCase(c)) {
                termLines.clear();
            } else if ("ls".equalsIgnoreCase(c)) {
                termLines.add("apps/  data/  etc/  home/  system/  bin/");
            } else if ("pwd".equalsIgnoreCase(c)) {
                termLines.add(termCwd);
            } else if ("uname -a".equalsIgnoreCase(c) || "uname".equalsIgnoreCase(c)) {
                termLines.add("Linux onuron-s25 6.6.21-onuron #1 SMP PREEMPT aarch64 GNU/Linux");
            } else if ("help".equalsIgnoreCase(c)) {
                termLines.add("Available commands: ls, pwd, uname, clear, help, date");
            } else if ("date".equalsIgnoreCase(c)) {
                termLines.add(new Date().toString());
            } else {
                termLines.add("nilshell: command executed: " + c);
            }
            invalidate();
        }

        public void sendMessage(String msg) {
            SimpleDateFormat sdf = new SimpleDateFormat("HH:mm", Locale.getDefault());
            activeChatMessages.add(new String[]{"me", msg, sdf.format(new Date())});
            invalidate();
        }

        private void drawAppVectorGlyph(Canvas canvas, float cx, float cy, String id, int color) {
            paint.setColor(color);
            paint.setStyle(Paint.Style.STROKE);
            paint.setStrokeWidth(dp(1.8f));
            paint.setStrokeCap(Paint.Cap.ROUND);
            paint.setStrokeJoin(Paint.Join.ROUND);

            switch (id) {
                case "app_phone":
                    android.graphics.Path phonePath = new android.graphics.Path();
                    phonePath.moveTo(cx - dp(6), cy - dp(9));
                    phonePath.lineTo(cx - dp(2), cy - dp(9));
                    phonePath.lineTo(cx - dp(4), cy - dp(4));
                    phonePath.lineTo(cx + dp(4), cy + dp(4));
                    phonePath.lineTo(cx + dp(9), cy + dp(2));
                    phonePath.lineTo(cx + dp(9), cy + dp(6));
                    canvas.drawPath(phonePath, paint);
                    break;
                case "app_browser":
                    canvas.drawCircle(cx, cy, dp(10), paint);
                    canvas.drawLine(cx - dp(10), cy, cx + dp(10), cy, paint);
                    canvas.drawOval(new RectF(cx - dp(4), cy - dp(10), cx + dp(4), cy + dp(10)), paint);
                    break;
                case "app_messages":
                    canvas.drawRoundRect(new RectF(cx - dp(10), cy - dp(8), cx + dp(10), cy + dp(6)), dp(4), dp(4), paint);
                    android.graphics.Path tail = new android.graphics.Path();
                    tail.moveTo(cx - dp(4), cy + dp(6));
                    tail.lineTo(cx - dp(7), cy + dp(10));
                    tail.lineTo(cx - dp(1), cy + dp(6));
                    canvas.drawPath(tail, paint);
                    break;
                case "app_files":
                    android.graphics.Path folder = new android.graphics.Path();
                    folder.moveTo(cx - dp(10), cy - dp(7));
                    folder.lineTo(cx - dp(4), cy - dp(7));
                    folder.lineTo(cx - dp(2), cy - dp(4));
                    folder.lineTo(cx + dp(10), cy - dp(4));
                    folder.lineTo(cx + dp(10), cy + dp(8));
                    folder.lineTo(cx - dp(10), cy + dp(8));
                    folder.close();
                    canvas.drawPath(folder, paint);
                    break;
                case "app_settings":
                    canvas.drawLine(cx - dp(9), cy - dp(4), cx + dp(9), cy - dp(4), paint);
                    canvas.drawLine(cx - dp(9), cy + dp(4), cx + dp(9), cy + dp(4), paint);
                    paint.setStyle(Paint.Style.FILL);
                    canvas.drawCircle(cx - dp(3), cy - dp(4), dp(2.5f), paint);
                    canvas.drawCircle(cx + dp(3), cy + dp(4), dp(2.5f), paint);
                    break;
                case "app_terminal":
                    android.graphics.Path prompt = new android.graphics.Path();
                    prompt.moveTo(cx - dp(7), cy - dp(5));
                    prompt.lineTo(cx - dp(3), cy - dp(1));
                    prompt.lineTo(cx - dp(7), cy + dp(3));
                    canvas.drawPath(prompt, paint);
                    canvas.drawLine(cx - dp(1), cy + dp(3), cx + dp(7), cy + dp(3), paint);
                    break;
                case "app_softbus":
                    android.graphics.Path delta = new android.graphics.Path();
                    delta.moveTo(cx, cy - dp(8));
                    delta.lineTo(cx + dp(8), cy + dp(6));
                    delta.lineTo(cx - dp(8), cy + dp(6));
                    delta.close();
                    canvas.drawPath(delta, paint);
                    paint.setStyle(Paint.Style.FILL);
                    canvas.drawCircle(cx, cy, dp(2f), paint);
                    break;
                case "app_camera":
                    canvas.drawRoundRect(new RectF(cx - dp(10), cy - dp(6), cx + dp(10), cy + dp(8)), dp(3), dp(3), paint);
                    canvas.drawCircle(cx, cy + dp(1), dp(4), paint);
                    canvas.drawRect(new RectF(cx - dp(4), cy - dp(9), cx + dp(4), cy - dp(6)), paint);
                    break;
                case "app_notes":
                    canvas.drawRoundRect(new RectF(cx - dp(8), cy - dp(9), cx + dp(8), cy + dp(9)), dp(2), dp(2), paint);
                    canvas.drawLine(cx - dp(5), cy - dp(4), cx + dp(5), cy - dp(4), paint);
                    canvas.drawLine(cx - dp(5), cy, cx + dp(5), cy, paint);
                    canvas.drawLine(cx - dp(5), cy + dp(4), cx + dp(1), cy + dp(4), paint);
                    break;
                case "app_music":
                    android.graphics.Path note = new android.graphics.Path();
                    note.moveTo(cx - dp(3), cy + dp(5));
                    note.lineTo(cx - dp(3), cy - dp(6));
                    note.lineTo(cx + dp(5), cy - dp(4));
                    note.lineTo(cx + dp(5), cy + dp(7));
                    canvas.drawPath(note, paint);
                    paint.setStyle(Paint.Style.FILL);
                    canvas.drawCircle(cx - dp(5), cy + dp(5), dp(3), paint);
                    canvas.drawCircle(cx + dp(3), cy + dp(7), dp(3), paint);
                    break;
                case "app_calculator":
                    canvas.drawRoundRect(new RectF(cx - dp(8), cy - dp(10), cx + dp(8), cy + dp(10)), dp(3), dp(3), paint);
                    canvas.drawLine(cx - dp(5), cy - dp(5), cx + dp(5), cy - dp(5), paint);
                    paint.setStyle(Paint.Style.FILL);
                    canvas.drawCircle(cx - dp(3), cy, dp(1.2f), paint);
                    canvas.drawCircle(cx + dp(3), cy, dp(1.2f), paint);
                    canvas.drawCircle(cx - dp(3), cy + dp(5), dp(1.2f), paint);
                    canvas.drawCircle(cx + dp(3), cy + dp(5), dp(1.2f), paint);
                    break;
                case "app_nilpkg":
                    canvas.drawRect(new RectF(cx - dp(8), cy - dp(6), cx + dp(8), cy + dp(8)), paint);
                    canvas.drawLine(cx - dp(8), cy - dp(6), cx, cy - dp(10), paint);
                    canvas.drawLine(cx + dp(8), cy - dp(6), cx, cy - dp(10), paint);
                    break;
                default:
                    canvas.drawCircle(cx, cy, dp(8), paint);
                    break;
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
    }
}
