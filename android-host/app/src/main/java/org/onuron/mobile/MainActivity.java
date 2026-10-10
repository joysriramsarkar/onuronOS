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
 * MainActivity — Unified Onuron OS Mobile Host Runtime on Samsung Galaxy S25
 * Full ArkUI/Material 3 styled system apps: Phone, Messages, Browser, Files,
 * Camera, Notes, Music, Calculator, Settings, and interactive Terminal.
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

            // 4. Floating Messages Input Bar
            setupMessageInputBar();
            FrameLayout.LayoutParams msgLp = new FrameLayout.LayoutParams(
                    ViewGroup.LayoutParams.MATCH_PARENT, ViewGroup.LayoutParams.WRAP_CONTENT);
            msgLp.gravity = Gravity.BOTTOM;
            msgLp.bottomMargin = (int) (56 * getResources().getDisplayMetrics().density);
            rootLayout.addView(messageInputBar, msgLp);

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
        terminalInputField.setHint("কমান্ড লিখুন (e.g. ls, pwd, cd, python)...");
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
        messageInputField.setHint("বার্তা লিখুন (Type message)...");
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
        sendBtn.setText("✈️");
        sendBtn.setTextColor(0xFF0A0E17);
        sendBtn.setBackgroundColor(0xFF00E5FF);
        sendBtn.setTextSize(16);
        sendBtn.setOnClickListener(v -> {
            String msg = messageInputField.getText().toString().trim();
            if (!msg.isEmpty() && onuronView != null) {
                onuronView.sendMessage(msg);
                messageInputField.setText("");
            }
        });
        messageInputBar.addView(sendBtn, new LinearLayout.LayoutParams(
                (int) (52 * getResources().getDisplayMetrics().density), (int) (38 * getResources().getDisplayMetrics().density)));
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
                SimpleDateFormat sdf = new SimpleDateFormat("HH:mm, dd MMM", Locale.getDefault());
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
                Toast.makeText(this, "📷 ক্যামেরা অ্যাপ্লিকেশন চালু হচ্ছে...", Toast.LENGTH_SHORT).show();
            }
        } catch (Exception e) {
            Log.e(TAG, "Camera launch error", e);
            Toast.makeText(this, "ক্যামেরা প্রস্তুত করা হচ্ছে...", Toast.LENGTH_SHORT).show();
        }
    }

    public void sendRealSms(String number, String message) {
        if (number == null || number.isEmpty() || message == null || message.isEmpty()) return;
        try {
            Intent smsIntent = new Intent(Intent.ACTION_SENDTO);
            smsIntent.setData(Uri.parse("smsto:" + number));
            smsIntent.putExtra("sms_body", message);
            startActivity(smsIntent);
        } catch (Exception e) {
            Log.e(TAG, "SMS launch error", e);
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
            setMessageInputVisible(false);
            onuronView.invalidate();
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

        enum Screen {
            HOME, PHONE, MESSAGES, FILES, SETTINGS, TERMINAL, ABOUT,
            NOTES, MUSIC, CALCULATOR
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
        public boolean wifiEnabled = true;
        public int wifiSignalLevel = 3; // 1 to 3
        public boolean cellularEnabled = true;
        public int cellularSignalLevel = 4; // 1 to 4
        public int batteryLevel = 95;
        public boolean isCharging = false;
        public boolean darkMode = true;
        public boolean vibratorEnabled = true;
        public int soundVolume = 75; // 0, 25, 50, 75, 100

        // Phone App State (ArkUI Tabbed: 0: Dialpad, 1: Contacts, 2: Call Logs)
        public int phoneTab = 0;
        public String dialNumber = "";
        public List<String[]> contacts = new ArrayList<>(); // [Name, Number]
        public List<String[]> callLogs = new ArrayList<>(); // [Number, Time, Status]

        // Messages App State
        public List<String[]> messageThreads = new ArrayList<>(); // [Sender, LastMsg, Time]
        public List<String[]> activeChatMessages = new ArrayList<>(); // [Sender, Msg, Time]
        public String activeRecipient = "Onuron System";
        public boolean inActiveChat = false;

        // Files App State
        public String currentPath;
        public String selectedFileInfo = null;

        // Terminal App State
        public List<String> termLines = new ArrayList<>();
        public String termCwd;

        // Calculator App State
        public String calcDisplay = "0";
        public String calcEquation = "";
        public double calcFirstOperand = 0;
        public String calcPendingOp = "";
        public boolean calcNewNumber = true;

        // Notes App State (aligned with notes-app-nilLang)
        public List<String[]> notesList = new ArrayList<>(); // [ID, Title, Snippet, Category, Priority, Date]
        public String selectedNoteDetail = null;

        // Music App State (aligned with music-streaming-nilLang)
        public boolean musicPlaying = false;
        public int musicCurrentTrack = 0;
        public int musicProgressSec = 105;
        public List<String[]> musicTracks = new ArrayList<>(); // [Title, Artist, Duration]

        public OnuronView(Context context, MainActivity activity) {
            super(context);
            this.activity = activity;
            this.vibrator = (Vibrator) context.getSystemService(Context.VIBRATOR_SERVICE);

            File appFiles = context.getFilesDir();
            this.termCwd = appFiles.getAbsolutePath();
            this.currentPath = appFiles.getAbsolutePath();

            try {
                new File(appFiles, "system.conf").createNewFile();
                new File(appFiles, "onuron_apps").mkdir();
                new File(appFiles, "downloads").mkdir();
                new File(appFiles, "notes").mkdir();
            } catch (Exception ignored) {
            }

            // Seed Contacts
            contacts.add(new String[]{"মা (Mom)", "+8801700000001"});
            contacts.add(new String[]{"বাবা (Dad)", "+8801700000002"});
            contacts.add(new String[]{"ইমার্জেন্সি হেল্পলাইন", "112"});
            contacts.add(new String[]{"অনুরণ ওএস সাপোর্ট", "+8801700000003"});

            // Seed Call Logs
            callLogs.add(new String[]{"+8801700000001", "১২:৪০, আজ", "আউটগোয়িং 📞"});
            callLogs.add(new String[]{"112", "১০:১৫, গতকাল", "সফল কল ✅"});
            callLogs.add(new String[]{"+8801700000003", "০৯:০০, ৮ অক্টো", "ইনকামিং 📲"});

            // Seed Messages
            messageThreads.add(new String[]{"Onuron System", "fscrypt v2 মেমরি এনক্রিপশন সক্রিয় রয়েছে।", "১২:৪৫"});
            messageThreads.add(new String[]{"মা (Mom)", "কেমন আছো? সময়মতো খেয়ে নিও।", "১২:৪০"});
            messageThreads.add(new String[]{"SoftBus Mesh", "NilPad-Pro-X1 সফলভাবে সংযুক্ত হয়েছে।", "১১:৩০"});

            activeChatMessages.add(new String[]{"Onuron System", "স্বাগতম Onuron OS মেসেজিং সেন্টারে!", "১২:০০"});
            activeChatMessages.add(new String[]{"Onuron System", "সিস্টেম কার্নেল ও ডিস্ট্রিবিউটেড সফটবাস সক্রিয়।", "১২:০১"});
            activeChatMessages.add(new String[]{"me", "ধন্যবাদ! সিস্টেম টেস্ট চলছে।", "১২:০৫"});

            // Seed Notes (from notes-app-nilLang)
            notesList.add(new String[]{"note-1", "স্বাগতম নীলাং নোটবুক-এ!", "নীলাং ভাষায় তৈরি আধুনিক, দ্রুতগতির নোট অ্যাপ্লিকেশন। সম্পূর্ণ বাংলায় ও ইংরেজিতে নিরাপদ সংরক্ষণ।", "গাইড", "High", "১০ অক্টো"});
            notesList.add(new String[]{"note-2", "দৈনিক কাজের পরিকল্পনা", "বিল্ড ও টেস্ট ভ্যালিডেশন সমাপ্ত করা, S25 রানটাইম যাচাই করা এবং গিট পুশ নিশ্চিত করা।", "কাজ", "High", "১০ অক্টো"});
            notesList.add(new String[]{"note-3", "অনুরণ ওএস আর্কিটেকচার", "Linux LTS → nilinit → NilHAL → Onuron daemons → nilrt → NilUI/Alap।", "সিস্টেম", "Normal", "০৯ অক্টো"});

            // Seed Music Tracks (from music-streaming-nilLang)
            musicTracks.add(new String[]{"Cyber Bangla 2040 (NilOS Theme)", "NilLang Sonic Lab", "03:50"});
            musicTracks.add(new String[]{"Onuron Ambient Waves", "Alap Audio Engine", "04:12"});
            musicTracks.add(new String[]{"Snapdragon 8 Elite Groove", "Qualcomm DSP Team", "02:45"});
            musicTracks.add(new String[]{"Alap Reactive Symphony", "Onuron Native Sound", "05:01"});

            // Initialize terminal banner
            termLines.add("==================================================");
            termLines.add("   Onuron OS Terminal CLI (Samsung Galaxy S25)   ");
            termLines.add("   Hardware: Snapdragon 8 Elite • Android Host   ");
            termLines.add("   Commands: cd, pwd, ls, mkdir, cat, python, clear");
            termLines.add("==================================================");
            termLines.add("Working Dir: " + termCwd);
            termLines.add("Ready. Type commands below or tap quick chips.");

            checkSystemNetwork();
        }

        // ── Dynamic Theme Palette ──
        public int getBgColor() { return darkMode ? 0xFF0A0E17 : 0xFFF1F5F9; }
        public int getSurfaceColor() { return darkMode ? 0xFF141C2B : 0xFFFFFFFF; }
        public int getSurfaceAltColor() { return darkMode ? 0xFF1E293B : 0xFFE2E8F0; }
        public int getBorderColor() { return darkMode ? 0xFF2D3748 : 0xFFCBD5E1; }
        public int getTextHighColor() { return darkMode ? 0xFFFFFFFF : 0xFF0F172A; }
        public int getTextMedColor() { return darkMode ? 0xFF94A3B8 : 0xFF475569; }
        public int getTextDimColor() { return darkMode ? 0xFF475569 : 0xFF94A3B8; }

        public static final int COLOR_CYAN = 0xFF00E5FF;
        public static final int COLOR_BLUE = 0xFF2979FF;
        public static final int COLOR_GREEN = 0xFF00E676;
        public static final int COLOR_AMBER = 0xFFFFB300;
        public static final int COLOR_PURPLE = 0xFF7C4DFF;
        public static final int COLOR_RED = 0xFFFF5252;

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

            // 1. Dynamic Canvas Background
            paint.setStyle(Paint.Style.FILL);
            paint.setColor(getBgColor());
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
                case CALCULATOR:
                    renderCalculatorScreen(canvas, w, h);
                    break;
                case NOTES:
                    renderNotesScreen(canvas, w, h);
                    break;
                case MUSIC:
                    renderMusicScreen(canvas, w, h);
                    break;
                case ABOUT:
                    renderAboutScreen(canvas, w, h);
                    break;
            }

            // 3. Top Dynamic Status Bar (Universal overlay)
            renderStatusBar(canvas, w);

            // 4. Bottom Navigation Bar
            renderBottomNav(canvas, w, h);
        }

        // ─── Status Bar ─────────────────────────────────────────────────────────────

        private void renderStatusBar(Canvas canvas, int w) {
            int barHeight = dp(42);

            paint.setStyle(Paint.Style.FILL);
            paint.setColor(darkMode ? 0xEE080C14 : 0xEEF1F5F9);
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
            paint.setColor(Color.BLACK);
            canvas.drawRoundRect(pillX, pillY, pillX + pillW, pillY + pillH, pillH / 2f, pillH / 2f, paint);

            paint.setColor(0xFF1E293B);
            canvas.drawCircle(pillX + dp(18), pillY + pillH / 2f, dp(4), paint);
            paint.setColor(0xFF0F172A);
            canvas.drawCircle(pillX + pillW - dp(24), pillY + pillH / 2f, dp(5), paint);

            touchAreas.add(new TouchArea(new RectF(pillX, 0, pillX + pillW, barHeight), "toggle_island"));

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
                    paint.setColor(i < cellularSignalLevel ? getTextHighColor() : (darkMode ? 0xFF2D3748 : 0xFFCBD5E1));
                    canvas.drawRoundRect(bx, by, bx + dp(2.5f), towerBaseY, dp(1), dp(1), paint);
                }
            } else {
                paint.setColor(darkMode ? 0xFF2D3748 : 0xFFCBD5E1);
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
            String netBadge = cellularEnabled ? (cellularSignalLevel >= 3 ? "৫G" : (cellularSignalLevel == 2 ? "৪G" : "৩G")) : "অফ";
            int netColor = cellularEnabled ? (cellularSignalLevel >= 3 ? COLOR_CYAN : COLOR_AMBER) : getTextDimColor();
            paint.setColor(netColor);
            paint.setTextSize(dp(11));
            paint.setTextAlign(Paint.Align.RIGHT);
            paint.setFakeBoldText(true);
            canvas.drawText(netBadge, rightX, dp(25), paint);

            touchAreas.add(new TouchArea(new RectF(rightX - dp(24), 0, towerX + towerW, barHeight), "cycle_signal"));
            rightX -= dp(28);

            // 3. Dynamic Vector Wi-Fi (3 concentric vector arcs + base dot)
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
            } else {
                paint.setStyle(Paint.Style.STROKE);
                paint.setStrokeWidth(dp(1.6f));
                paint.setColor(darkMode ? 0xFF2D3748 : 0xFFCBD5E1);
                canvas.drawArc(new RectF(wifiX, dp(11.5f), wifiX + wifiW, dp(29)), 220, 100, false, paint);
                paint.setStyle(Paint.Style.FILL);
                paint.setColor(COLOR_RED);
                paint.setTextSize(dp(9));
                paint.setTextAlign(Paint.Align.CENTER);
                canvas.drawText("✕", wifiX + wifiW / 2f, dp(25), paint);
            }

            touchAreas.add(new TouchArea(new RectF(wifiX - dp(6), 0, rightX + dp(4), barHeight), "toggle_wifi"));
        }

        // ─── Home Screen (ArkUI/Material 3 Grid: 12 Apps) ───────────────────────────

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
                    new AppItem("অনুরণ", "app_about", "ℹ️", COLOR_CYAN),
            };

            for (int i = 0; i < apps.length; i++) {
                int r = i / cols;
                int c = i % cols;
                float cx = dp(16) + c * colWidth + colWidth / 2f;
                float cy = gridY + r * dp(90);

                RectF iconRect = new RectF(cx - iconSize / 2f, cy, cx + iconSize / 2f, cy + iconSize);
                paint.setStyle(Paint.Style.FILL);
                paint.setColor(getSurfaceColor());
                canvas.drawRoundRect(iconRect, dp(16), dp(16), paint);

                paint.setStyle(Paint.Style.STROKE);
                paint.setColor(getBorderColor());
                paint.setStrokeWidth(dp(1));
                canvas.drawRoundRect(iconRect, dp(16), dp(16), paint);

                paint.setStyle(Paint.Style.FILL);
                paint.setColor(apps[i].accent);
                canvas.drawCircle(cx, cy + dp(12), dp(3f), paint);

                paint.setTextSize(dp(22));
                paint.setTextAlign(Paint.Align.CENTER);
                canvas.drawText(apps[i].emoji, cx, cy + dp(40), paint);

                paint.setColor(getTextHighColor());
                paint.setTextSize(dp(12));
                paint.setFakeBoldText(false);
                canvas.drawText(apps[i].title, cx, cy + iconSize + dp(18), paint);

                touchAreas.add(new TouchArea(iconRect, apps[i].id));
            }
        }

        // ─── Phone App (ArkUI Tabbed: Keypad, Contacts, Recent Calls) ───────────────

        private void renderPhoneScreen(Canvas canvas, int w, int h) {
            float y = dp(52);
            paint.setColor(COLOR_GREEN);
            paint.setTextSize(dp(20));
            paint.setFakeBoldText(true);
            paint.setTextAlign(Paint.Align.LEFT);
            canvas.drawText("ফোন (Phone & Dialer)", dp(20), y + dp(20), paint);

            // Tab Bar: [ ডায়ালপ্যাড ] | [ কন্টাক্টস ] | [ কল হিস্ট্রি ]
            y += dp(32);
            float tabW = (w - dp(40)) / 3f;
            String[] tabs = {"ডায়ালপ্যাড", "কন্টাক্টস (" + contacts.size() + ")", "কল লগ (" + callLogs.size() + ")"};
            for (int i = 0; i < 3; i++) {
                RectF tRect = new RectF(dp(20) + i * tabW, y, dp(20) + (i + 1) * tabW - dp(6), y + dp(34));
                paint.setStyle(Paint.Style.FILL);
                paint.setColor(phoneTab == i ? (darkMode ? 0xFF064E3B : 0xFFDCFCE7) : getSurfaceColor());
                canvas.drawRoundRect(tRect, dp(8), dp(8), paint);

                paint.setColor(phoneTab == i ? COLOR_GREEN : getTextMedColor());
                paint.setTextSize(dp(11.5f));
                paint.setTextAlign(Paint.Align.CENTER);
                canvas.drawText(tabs[i], tRect.centerX(), y + dp(22), paint);

                touchAreas.add(new TouchArea(tRect, "phone_tab_" + i));
            }

            y += dp(44);

            if (phoneTab == 0) {
                // Number card
                RectF numCard = new RectF(dp(20), y, w - dp(20), y + dp(56));
                paint.setStyle(Paint.Style.FILL);
                paint.setColor(getSurfaceColor());
                canvas.drawRoundRect(numCard, dp(12), dp(12), paint);
                paint.setStyle(Paint.Style.STROKE);
                paint.setColor(getBorderColor());
                canvas.drawRoundRect(numCard, dp(12), dp(12), paint);

                paint.setStyle(Paint.Style.FILL);
                paint.setColor(getTextHighColor());
                paint.setTextSize(dp(22));
                paint.setTextAlign(Paint.Align.CENTER);
                String display = dialNumber.isEmpty() ? "নম্বর লিখুন..." : toBengaliDigits(dialNumber);
                canvas.drawText(display, w / 2f, y + dp(36), paint);

                // Keypad grid
                float padStartY = y + dp(68);
                float btnW = (w - dp(64)) / 3f;
                float btnH = dp(52);
                String[][] pad = {
                        {"1", "2", "3"},
                        {"4", "5", "6"},
                        {"7", "8", "9"},
                        {"*", "0", "#"}
                };

                for (int r = 0; r < 4; r++) {
                    for (int c = 0; c < 3; c++) {
                        float bx = dp(20) + c * (btnW + dp(12));
                        float by = padStartY + r * (btnH + dp(10));
                        RectF btnRect = new RectF(bx, by, bx + btnW, by + btnH);

                        paint.setStyle(Paint.Style.FILL);
                        paint.setColor(getSurfaceAltColor());
                        canvas.drawRoundRect(btnRect, dp(12), dp(12), paint);
                        paint.setStyle(Paint.Style.STROKE);
                        paint.setColor(getBorderColor());
                        canvas.drawRoundRect(btnRect, dp(12), dp(12), paint);

                        paint.setStyle(Paint.Style.FILL);
                        paint.setColor(getTextHighColor());
                        paint.setTextSize(dp(20));
                        paint.setTextAlign(Paint.Align.CENTER);
                        canvas.drawText(toBengaliDigits(pad[r][c]), bx + btnW / 2f, by + btnH / 2f + dp(7), paint);

                        touchAreas.add(new TouchArea(btnRect, "key_" + pad[r][c]));
                    }
                }

                // Action buttons: [ 📞 কল করুন ] [ 💾 সেভ ] [ ⌫ ]
                float actY = padStartY + 4 * (btnH + dp(10)) + dp(8);
                float callBtnW = w - dp(140);
                RectF callRect = new RectF(dp(20), actY, dp(20) + callBtnW, actY + dp(50));
                paint.setStyle(Paint.Style.FILL);
                paint.setColor(COLOR_GREEN);
                canvas.drawRoundRect(callRect, dp(14), dp(14), paint);
                paint.setColor(0xFF0A0E17);
                paint.setTextSize(dp(16));
                paint.setFakeBoldText(true);
                canvas.drawText("📞 কল করুন (Call)", callRect.centerX(), actY + dp(32), paint);
                touchAreas.add(new TouchArea(callRect, "act_call"));

                RectF saveRect = new RectF(dp(28) + callBtnW, actY, dp(28) + callBtnW + dp(48), actY + dp(50));
                paint.setColor(getSurfaceAltColor());
                canvas.drawRoundRect(saveRect, dp(14), dp(14), paint);
                paint.setColor(COLOR_CYAN);
                paint.setTextSize(dp(18));
                canvas.drawText("💾", saveRect.centerX(), actY + dp(32), paint);
                touchAreas.add(new TouchArea(saveRect, "act_save_contact"));

                RectF delRect = new RectF(w - dp(56), actY, w - dp(20), actY + dp(50));
                paint.setColor(getSurfaceAltColor());
                canvas.drawRoundRect(delRect, dp(14), dp(14), paint);
                paint.setColor(COLOR_RED);
                paint.setTextSize(dp(18));
                canvas.drawText("⌫", delRect.centerX(), actY + dp(32), paint);
                touchAreas.add(new TouchArea(delRect, "act_del"));

            } else if (phoneTab == 1) {
                // Contacts List
                for (int i = 0; i < contacts.size(); i++) {
                    String[] c = contacts.get(i);
                    RectF card = new RectF(dp(20), y, w - dp(20), y + dp(56));
                    paint.setStyle(Paint.Style.FILL);
                    paint.setColor(getSurfaceColor());
                    canvas.drawRoundRect(card, dp(12), dp(12), paint);

                    // Avatar Circle
                    paint.setColor(COLOR_BLUE);
                    canvas.drawCircle(dp(44), y + dp(28), dp(18), paint);
                    paint.setColor(Color.WHITE);
                    paint.setTextSize(dp(14));
                    paint.setTextAlign(Paint.Align.CENTER);
                    canvas.drawText(c[0].substring(0, 1), dp(44), y + dp(33), paint);

                    // Name and Number
                    paint.setColor(getTextHighColor());
                    paint.setTextAlign(Paint.Align.LEFT);
                    paint.setTextSize(dp(14));
                    paint.setFakeBoldText(true);
                    canvas.drawText(c[0], dp(74), y + dp(25), paint);

                    paint.setColor(getTextMedColor());
                    paint.setTextSize(dp(12));
                    paint.setFakeBoldText(false);
                    canvas.drawText(toBengaliDigits(c[1]), dp(74), y + dp(44), paint);

                    // Call icon
                    paint.setColor(COLOR_GREEN);
                    paint.setTextSize(dp(16));
                    paint.setTextAlign(Paint.Align.RIGHT);
                    canvas.drawText("📞", w - dp(36), y + dp(34), paint);

                    touchAreas.add(new TouchArea(card, "call_contact_" + i));
                    y += dp(66);
                }
            } else {
                // Recent Call Logs
                for (int i = 0; i < callLogs.size(); i++) {
                    String[] log = callLogs.get(i);
                    RectF card = new RectF(dp(20), y, w - dp(20), y + dp(52));
                    paint.setStyle(Paint.Style.FILL);
                    paint.setColor(getSurfaceColor());
                    canvas.drawRoundRect(card, dp(12), dp(12), paint);

                    paint.setColor(getTextHighColor());
                    paint.setTextAlign(Paint.Align.LEFT);
                    paint.setTextSize(dp(13.5f));
                    paint.setFakeBoldText(true);
                    canvas.drawText(toBengaliDigits(log[0]), dp(34), y + dp(24), paint);

                    paint.setColor(getTextMedColor());
                    paint.setTextSize(dp(11.5f));
                    paint.setFakeBoldText(false);
                    canvas.drawText(log[1] + " • " + log[2], dp(34), y + dp(42), paint);

                    paint.setColor(COLOR_GREEN);
                    paint.setTextAlign(Paint.Align.RIGHT);
                    canvas.drawText("📞", w - dp(34), y + dp(32), paint);

                    touchAreas.add(new TouchArea(card, "redial_log_" + i));
                    y += dp(60);
                }
            }
        }

        // ─── Messages Screen (Direct Interactive Messaging) ─────────────────────────

        private void renderMessagesScreen(Canvas canvas, int w, int h) {
            float y = dp(52);
            paint.setColor(COLOR_AMBER);
            paint.setTextSize(dp(20));
            paint.setFakeBoldText(true);
            paint.setTextAlign(Paint.Align.LEFT);

            if (!inActiveChat) {
                canvas.drawText("বার্তা (Messages SMS)", dp(20), y + dp(20), paint);

                // + New Chat Button
                RectF newBtn = new RectF(w - dp(120), y, w - dp(20), y + dp(32));
                paint.setStyle(Paint.Style.FILL);
                paint.setColor(COLOR_AMBER);
                canvas.drawRoundRect(newBtn, dp(8), dp(8), paint);
                paint.setColor(0xFF0A0E17);
                paint.setTextSize(dp(12));
                paint.setTextAlign(Paint.Align.CENTER);
                canvas.drawText("+ নতুন বার্তা", newBtn.centerX(), y + dp(21), paint);
                touchAreas.add(new TouchArea(newBtn, "msg_new_chat"));

                y += dp(44);

                for (int i = 0; i < messageThreads.size(); i++) {
                    String[] th = messageThreads.get(i);
                    RectF card = new RectF(dp(20), y, w - dp(20), y + dp(62));
                    paint.setStyle(Paint.Style.FILL);
                    paint.setColor(getSurfaceColor());
                    canvas.drawRoundRect(card, dp(12), dp(12), paint);

                    paint.setColor(COLOR_CYAN);
                    paint.setTextSize(dp(14));
                    paint.setFakeBoldText(true);
                    paint.setTextAlign(Paint.Align.LEFT);
                    canvas.drawText(th[0], dp(34), y + dp(26), paint);

                    paint.setColor(getTextMedColor());
                    paint.setTextSize(dp(11));
                    paint.setTextAlign(Paint.Align.RIGHT);
                    canvas.drawText(toBengaliDigits(th[2]), w - dp(34), y + dp(26), paint);

                    paint.setColor(getTextHighColor());
                    paint.setTextSize(dp(12));
                    paint.setFakeBoldText(false);
                    paint.setTextAlign(Paint.Align.LEFT);
                    canvas.drawText(th[1], dp(34), y + dp(48), paint);

                    touchAreas.add(new TouchArea(card, "open_chat_" + i));
                    y += dp(72);
                }
            } else {
                // Active Chat View
                canvas.drawText("💬 " + activeRecipient, dp(48), y + dp(20), paint);

                // Back Button
                RectF backBtn = new RectF(dp(16), y, dp(44), y + dp(30));
                paint.setColor(getSurfaceAltColor());
                canvas.drawRoundRect(backBtn, dp(6), dp(6), paint);
                paint.setColor(COLOR_CYAN);
                paint.setTextSize(dp(14));
                paint.setTextAlign(Paint.Align.CENTER);
                canvas.drawText("◀", backBtn.centerX(), y + dp(21), paint);
                touchAreas.add(new TouchArea(backBtn, "chat_back"));

                y += dp(44);

                // Chat bubbles
                for (String[] m : activeChatMessages) {
                    boolean isMe = "me".equals(m[0]);
                    float bubbleW = w * 0.7f;
                    float bx = isMe ? (w - dp(20) - bubbleW) : dp(20);
                    RectF bubble = new RectF(bx, y, bx + bubbleW, y + dp(46));

                    paint.setStyle(Paint.Style.FILL);
                    paint.setColor(isMe ? (darkMode ? 0xFF065F46 : 0xFF10B981) : getSurfaceColor());
                    canvas.drawRoundRect(bubble, dp(12), dp(12), paint);

                    paint.setColor(Color.WHITE);
                    paint.setTextSize(dp(12.5f));
                    paint.setTextAlign(Paint.Align.LEFT);
                    canvas.drawText(m[1], bx + dp(12), y + dp(22), paint);

                    paint.setColor(isMe ? 0xFFD1FAE5 : getTextMedColor());
                    paint.setTextSize(dp(10));
                    paint.setTextAlign(Paint.Align.RIGHT);
                    canvas.drawText(toBengaliDigits(m[2]), bx + bubbleW - dp(10), y + dp(38), paint);

                    y += dp(56);
                }
            }
        }

        public void sendMessage(String text) {
            SimpleDateFormat sdf = new SimpleDateFormat("HH:mm", Locale.getDefault());
            String time = sdf.format(new Date());
            activeChatMessages.add(new String[]{"me", text, time});

            // Update threads
            if (!messageThreads.isEmpty()) {
                messageThreads.get(0)[1] = text;
                messageThreads.get(0)[2] = time;
            }

            activity.sendRealSms(activeRecipient, text);
            invalidate();
        }

        // ─── Files App (Interactive Folder Navigation & File Detail) ────────────────

        private void renderFilesScreen(Canvas canvas, int w, int h) {
            float y = dp(52);
            paint.setColor(COLOR_BLUE);
            paint.setTextSize(dp(20));
            paint.setFakeBoldText(true);
            paint.setTextAlign(Paint.Align.LEFT);
            canvas.drawText("ফাইল ম্যানেজার (Files)", dp(20), y + dp(20), paint);

            // + New File Button
            RectF newFileBtn = new RectF(w - dp(120), y, w - dp(20), y + dp(32));
            paint.setStyle(Paint.Style.FILL);
            paint.setColor(COLOR_BLUE);
            canvas.drawRoundRect(newFileBtn, dp(8), dp(8), paint);
            paint.setColor(Color.WHITE);
            paint.setTextSize(dp(12));
            paint.setTextAlign(Paint.Align.CENTER);
            canvas.drawText("+ নতুন ফাইল", newFileBtn.centerX(), y + dp(21), paint);
            touchAreas.add(new TouchArea(newFileBtn, "files_new_file"));

            y += dp(36);
            paint.setColor(getTextMedColor());
            paint.setTextSize(dp(11.5f));
            paint.setTextAlign(Paint.Align.LEFT);
            canvas.drawText("পাথ: " + currentPath, dp(20), y + dp(12), paint);

            y += dp(24);

            // If not in root filesDir, provide Up/Back folder button
            File dir = new File(currentPath);
            File parent = dir.getParentFile();
            if (parent != null && parent.exists() && !currentPath.equals(getContext().getFilesDir().getParent())) {
                RectF upCard = new RectF(dp(20), y, w - dp(20), y + dp(38));
                paint.setStyle(Paint.Style.FILL);
                paint.setColor(getSurfaceAltColor());
                canvas.drawRoundRect(upCard, dp(8), dp(8), paint);
                paint.setColor(COLOR_CYAN);
                paint.setTextSize(dp(12.5f));
                paint.setTextAlign(Paint.Align.LEFT);
                canvas.drawText("⬆️ [.. আগের ফোল্ডার / Up]", dp(34), y + dp(24), paint);
                touchAreas.add(new TouchArea(upCard, "file_go_up"));
                y += dp(46);
            }

            File[] files = dir.listFiles();
            if (files != null && files.length > 0) {
                int count = 0;
                for (int i = 0; i < files.length; i++) {
                    if (count++ > 7) break;
                    File f = files[i];
                    RectF card = new RectF(dp(20), y, w - dp(20), y + dp(46));
                    paint.setStyle(Paint.Style.FILL);
                    paint.setColor(getSurfaceColor());
                    canvas.drawRoundRect(card, dp(10), dp(10), paint);

                    String icon = f.isDirectory() ? "📁 [DIR] " : "📄 [FILE] ";
                    paint.setColor(f.isDirectory() ? COLOR_CYAN : getTextHighColor());
                    paint.setTextSize(dp(13));
                    paint.setTextAlign(Paint.Align.LEFT);
                    canvas.drawText(icon + f.getName(), dp(32), y + dp(28), paint);

                    paint.setColor(getTextMedColor());
                    paint.setTextSize(dp(11));
                    paint.setTextAlign(Paint.Align.RIGHT);
                    String sz = f.isDirectory() ? "ফোল্ডার" : (f.length() / 1024 + " KB");
                    canvas.drawText(sz, w - dp(32), y + dp(28), paint);

                    touchAreas.add(new TouchArea(card, "file_click_" + i));
                    y += dp(52);
                }
            } else {
                paint.setColor(getTextDimColor());
                paint.setTextSize(dp(13));
                paint.setTextAlign(Paint.Align.LEFT);
                canvas.drawText("এই ফোল্ডারে কোনো ফাইল নেই।", dp(20), y + dp(24), paint);
            }

            if (selectedFileInfo != null) {
                y += dp(10);
                RectF infoBox = new RectF(dp(20), y, w - dp(20), y + dp(50));
                paint.setColor(0xFF0F172A);
                canvas.drawRoundRect(infoBox, dp(8), dp(8), paint);
                paint.setColor(COLOR_CYAN);
                paint.setTextSize(dp(11.5f));
                canvas.drawText(selectedFileInfo, dp(30), y + dp(30), paint);
            }
        }

        // ─── Settings App (ArkUI/Material 3 Responsive Toggles) ─────────────────────

        private void renderSettingsScreen(Canvas canvas, int w, int h) {
            float y = dp(52);
            paint.setColor(COLOR_PURPLE);
            paint.setTextSize(dp(20));
            paint.setFakeBoldText(true);
            paint.setTextAlign(Paint.Align.LEFT);
            canvas.drawText("সেটিংস (Settings & Control)", dp(20), y + dp(20), paint);

            y += dp(36);

            SettingToggle[] toggles = {
                    new SettingToggle("ভাইব্রেশন ও হ্যাপটিক (Vibration)", vibratorEnabled ? "[ চালু ]" : "[ বন্ধ ]", vibratorEnabled ? COLOR_GREEN : getTextDimColor(), "toggle_vibration"),
                    new SettingToggle("ডার্ক মোড থিম (Theme)", darkMode ? "[ ডার্ক ]" : "[ লাইট ]", darkMode ? COLOR_GREEN : COLOR_AMBER, "toggle_dark"),
                    new SettingToggle("ওয়াইফাই নেটওয়ার্ক (Wi-Fi)", wifiEnabled ? "[ চালু • ৩/৩ ]" : "[ বন্ধ ]", wifiEnabled ? COLOR_CYAN : getTextDimColor(), "toggle_wifi"),
                    new SettingToggle("সেলুলার ৫G ডেটা (Cellular)", cellularEnabled ? "[ চালু • ৫G ]" : "[ বন্ধ ]", cellularEnabled ? COLOR_CYAN : getTextDimColor(), "toggle_cellular"),
                    new SettingToggle("সিগন্যাল ক্ষমতা (Signal Tower)", cellularSignalLevel + "/৪ বার ক্ষমতা", COLOR_CYAN, "cycle_signal"),
                    new SettingToggle("ব্যাটারি চার্জিং স্টেট", isCharging ? "[ চার্জ হচ্ছে ⚡ ]" : "[ সাধারণ " + batteryLevel + "% ]", isCharging ? COLOR_AMBER : COLOR_GREEN, "toggle_battery"),
                    new SettingToggle("ভলিউম ও সাউন্ড (Volume)", soundVolume == 0 ? "[ মিউট ]" : "[ " + soundVolume + "% ]", soundVolume > 0 ? COLOR_BLUE : COLOR_RED, "cycle_volume"),
                    new SettingToggle("সিস্টেম ভাষা (Language)", "[ বাংলা (Bangladesh) ]", COLOR_PURPLE, "toggle_lang"),
                    new SettingToggle("ডিভাইস পরিচিতি (About)", "Samsung S25 • 8 Elite", COLOR_CYAN, "app_about"),
            };

            for (SettingToggle t : toggles) {
                RectF card = new RectF(dp(20), y, w - dp(20), y + dp(48));
                paint.setStyle(Paint.Style.FILL);
                paint.setColor(getSurfaceColor());
                canvas.drawRoundRect(card, dp(12), dp(12), paint);

                paint.setStyle(Paint.Style.STROKE);
                paint.setColor(getBorderColor());
                paint.setStrokeWidth(dp(1));
                canvas.drawRoundRect(card, dp(12), dp(12), paint);

                paint.setStyle(Paint.Style.FILL);
                paint.setColor(getTextHighColor());
                paint.setTextSize(dp(13.5f));
                paint.setFakeBoldText(false);
                paint.setTextAlign(Paint.Align.LEFT);
                canvas.drawText(t.title, dp(34), y + dp(30), paint);

                paint.setColor(t.color);
                paint.setFakeBoldText(true);
                paint.setTextAlign(Paint.Align.RIGHT);
                canvas.drawText(t.status, w - dp(34), y + dp(30), paint);

                touchAreas.add(new TouchArea(card, t.id));
                y += dp(54);
            }
        }

        // ─── Terminal Screen (Fixed CD, Python, Dynamic Auto-Scroll) ────────────────

        private void renderTerminalScreen(Canvas canvas, int w, int h) {
            float y = dp(52);
            paint.setColor(COLOR_CYAN);
            paint.setTextSize(dp(20));
            paint.setFakeBoldText(true);
            paint.setTextAlign(Paint.Align.LEFT);
            canvas.drawText("টার্মিনাল (Terminal CLI)", dp(20), y + dp(20), paint);

            y += dp(32);

            // Quick Chips
            String[] chips = {"ls -la", "pwd", "cd ..", "uname -a", "python -c \"2+2\"", "clear"};
            float chipW = (w - dp(52)) / 3f;
            for (int i = 0; i < chips.length; i++) {
                int r = i / 3;
                int c = i % 3;
                float cx = dp(20) + c * (chipW + dp(6));
                float cy = y + r * dp(32);
                RectF chipRect = new RectF(cx, cy, cx + chipW, cy + dp(26));

                paint.setStyle(Paint.Style.FILL);
                paint.setColor(getSurfaceColor());
                canvas.drawRoundRect(chipRect, dp(8), dp(8), paint);

                paint.setColor(COLOR_CYAN);
                paint.setTextSize(dp(11.5f));
                paint.setTextAlign(Paint.Align.CENTER);
                canvas.drawText(chips[i], cx + chipW / 2f, cy + dp(17), paint);

                touchAreas.add(new TouchArea(chipRect, "chip_" + chips[i]));
            }

            y += dp(72);

            // Terminal Screen Output Box
            float boxBottom = h - dp(116);
            RectF termBox = new RectF(dp(20), y, w - dp(20), boxBottom);
            paint.setStyle(Paint.Style.FILL);
            paint.setColor(darkMode ? 0xFF040609 : 0xFFFFFFFF);
            canvas.drawRoundRect(termBox, dp(12), dp(12), paint);

            paint.setStyle(Paint.Style.STROKE);
            paint.setColor(getBorderColor());
            canvas.drawRoundRect(termBox, dp(12), dp(12), paint);

            // Render output lines with dynamic auto-scrolling
            paint.setStyle(Paint.Style.FILL);
            paint.setTextSize(dp(11.5f));
            paint.setTextAlign(Paint.Align.LEFT);

            float lineY = y + dp(20);
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
                    paint.setColor(getTextMedColor());
                }
                canvas.drawText(l, dp(30), lineY, paint);
                lineY += dp(18);
            }

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

            // Built-in cd command implementation (changes parent termCwd!)
            if (cmd.equals("cd") || cmd.startsWith("cd ")) {
                String target = cmd.length() > 3 ? cmd.substring(3).trim() : "";
                if (target.isEmpty() || target.equals("~")) {
                    termCwd = getContext().getFilesDir().getAbsolutePath();
                } else if (target.equals("..")) {
                    File parent = new File(termCwd).getParentFile();
                    if (parent != null && parent.exists() && parent.canRead()) {
                        termCwd = parent.getAbsolutePath();
                    }
                } else {
                    File next = target.startsWith("/") ? new File(target) : new File(termCwd, target);
                    if (next.exists() && next.isDirectory()) {
                        termCwd = next.getAbsolutePath();
                    } else {
                        termLines.add("! cd: " + target + ": No such directory");
                    }
                }
                termLines.add("Current: " + termCwd);
                invalidate();
                return;
            }

            // Built-in python engine
            if (cmd.equals("python") || cmd.equals("python3")) {
                termLines.add("Python 3.12.2 (Onuron MicroPython Engine, Oct 10 2026)");
                termLines.add("[Clang 18.0 ARM64 musl] on onuron_mobile");
                termLines.add("Type python -c \"<expr>\" for live evaluation.");
                invalidate();
                return;
            }

            if (cmd.startsWith("python -c ") || cmd.startsWith("python3 -c ")) {
                String code = cmd.substring(cmd.indexOf("-c") + 2).trim().replace("\"", "").replace("'", "");
                try {
                    // Evaluate simple arithmetic or print statements
                    if (code.startsWith("print(") && code.endsWith(")")) {
                        code = code.substring(6, code.length() - 1);
                    }
                    if (code.contains("+") || code.contains("-") || code.contains("*") || code.contains("/")) {
                        String[] parts;
                        if (code.contains("+")) {
                            parts = code.split("\\+");
                            double res = Double.parseDouble(parts[0].trim()) + Double.parseDouble(parts[1].trim());
                            termLines.add(String.valueOf(res));
                        } else if (code.contains("-")) {
                            parts = code.split("-");
                            double res = Double.parseDouble(parts[0].trim()) - Double.parseDouble(parts[1].trim());
                            termLines.add(String.valueOf(res));
                        } else if (code.contains("*")) {
                            parts = code.split("\\*");
                            double res = Double.parseDouble(parts[0].trim()) * Double.parseDouble(parts[1].trim());
                            termLines.add(String.valueOf(res));
                        } else {
                            parts = code.split("/");
                            double res = Double.parseDouble(parts[0].trim()) / Double.parseDouble(parts[1].trim());
                            termLines.add(String.valueOf(res));
                        }
                    } else {
                        termLines.add(code);
                    }
                } catch (Exception e) {
                    termLines.add("! Python syntax error: " + e.getMessage());
                }
                invalidate();
                return;
            }

            try {
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

            while (termLines.size() > 80) {
                termLines.remove(0);
            }
            invalidate();
        }

        // ─── Calculator App (ArkUI Styled Reactive Calculator) ──────────────────────

        private void renderCalculatorScreen(Canvas canvas, int w, int h) {
            float y = dp(52);
            paint.setColor(COLOR_CYAN);
            paint.setTextSize(dp(20));
            paint.setFakeBoldText(true);
            paint.setTextAlign(Paint.Align.LEFT);
            canvas.drawText("ক্যালকুলেটর (Calculator)", dp(20), y + dp(20), paint);

            y += dp(36);

            // Display Card
            RectF dispCard = new RectF(dp(20), y, w - dp(20), y + dp(76));
            paint.setStyle(Paint.Style.FILL);
            paint.setColor(getSurfaceColor());
            canvas.drawRoundRect(dispCard, dp(14), dp(14), paint);
            paint.setStyle(Paint.Style.STROKE);
            paint.setColor(getBorderColor());
            canvas.drawRoundRect(dispCard, dp(14), dp(14), paint);

            paint.setStyle(Paint.Style.FILL);
            paint.setColor(getTextMedColor());
            paint.setTextSize(dp(13));
            paint.setTextAlign(Paint.Align.RIGHT);
            canvas.drawText(calcEquation, w - dp(36), y + dp(28), paint);

            paint.setColor(getTextHighColor());
            paint.setTextSize(dp(28));
            paint.setFakeBoldText(true);
            canvas.drawText(toBengaliDigits(calcDisplay), w - dp(36), y + dp(62), paint);

            // Keypad
            float keyStartY = y + dp(90);
            float btnW = (w - dp(68)) / 4f;
            float btnH = dp(56);

            String[][] keys = {
                    {"C", "⌫", "%", "÷"},
                    {"7", "8", "9", "×"},
                    {"4", "5", "6", "-"},
                    {"1", "2", "3", "+"},
                    {"0", ".", "=", "="}
            };

            for (int r = 0; r < 5; r++) {
                for (int c = 0; c < 4; c++) {
                    if (r == 4 && c == 3) continue; // Skip redundant equal button

                    float bx = dp(20) + c * (btnW + dp(8));
                    float by = keyStartY + r * (btnH + dp(8));
                    float thisW = (r == 4 && c == 0) ? (btnW * 2 + dp(8)) : btnW;

                    if (r == 4 && c == 1) continue; // Span 0 over 2 slots
                    if (r == 4 && c == 2) {
                        bx = dp(20) + 2 * (btnW + dp(8));
                        thisW = btnW;
                    }

                    RectF btnRect = new RectF(bx, by, bx + thisW, by + btnH);
                    String k = keys[r][c];

                    paint.setStyle(Paint.Style.FILL);
                    boolean isOp = "÷".equals(k) || "×".equals(k) || "-".equals(k) || "+".equals(k);
                    boolean isEq = "=".equals(k);
                    paint.setColor(isEq ? COLOR_GREEN : (isOp ? (darkMode ? 0xFF0E7490 : 0xFFBAE6FD) : getSurfaceAltColor()));
                    canvas.drawRoundRect(btnRect, dp(12), dp(12), paint);

                    paint.setStyle(Paint.Style.STROKE);
                    paint.setColor(getBorderColor());
                    canvas.drawRoundRect(btnRect, dp(12), dp(12), paint);

                    paint.setStyle(Paint.Style.FILL);
                    paint.setColor(isEq ? 0xFF0A0E17 : (isOp ? COLOR_CYAN : getTextHighColor()));
                    paint.setTextSize(dp(20));
                    paint.setFakeBoldText(true);
                    paint.setTextAlign(Paint.Align.CENTER);
                    canvas.drawText(toBengaliDigits(k), btnRect.centerX(), by + btnH / 2f + dp(7), paint);

                    touchAreas.add(new TouchArea(btnRect, "calc_key_" + k));
                }
            }
        }

        private void handleCalc(String key) {
            switch (key) {
                case "C":
                    calcDisplay = "0";
                    calcEquation = "";
                    calcFirstOperand = 0;
                    calcPendingOp = "";
                    calcNewNumber = true;
                    break;
                case "⌫":
                    if (calcDisplay.length() > 1) {
                        calcDisplay = calcDisplay.substring(0, calcDisplay.length() - 1);
                    } else {
                        calcDisplay = "0";
                    }
                    break;
                case "+":
                case "-":
                case "×":
                case "÷":
                case "%":
                    try {
                        calcFirstOperand = Double.parseDouble(calcDisplay);
                        calcPendingOp = key;
                        calcEquation = calcDisplay + " " + key;
                        calcNewNumber = true;
                    } catch (Exception ignored) {
                    }
                    break;
                case "=":
                    try {
                        double second = Double.parseDouble(calcDisplay);
                        double res = 0;
                        if ("+".equals(calcPendingOp)) res = calcFirstOperand + second;
                        else if ("-".equals(calcPendingOp)) res = calcFirstOperand - second;
                        else if ("×".equals(calcPendingOp)) res = calcFirstOperand * second;
                        else if ("÷".equals(calcPendingOp)) res = second != 0 ? calcFirstOperand / second : 0;
                        else if ("%".equals(calcPendingOp)) res = calcFirstOperand * (second / 100.0);

                        calcEquation = calcEquation + " " + calcDisplay + " =";
                        if (res == (long) res) {
                            calcDisplay = String.valueOf((long) res);
                        } else {
                            calcDisplay = String.format(Locale.US, "%.4f", res);
                        }
                        calcNewNumber = true;
                    } catch (Exception ignored) {
                    }
                    break;
                default:
                    // Digits or dot
                    if (calcNewNumber) {
                        calcDisplay = key;
                        calcNewNumber = false;
                    } else {
                        if (".".equals(key) && calcDisplay.contains(".")) break;
                        calcDisplay += key;
                    }
            }
        }

        // ─── Notes App (Aligned with notes-app-nilLang) ─────────────────────────────

        private void renderNotesScreen(Canvas canvas, int w, int h) {
            float y = dp(52);
            paint.setColor(COLOR_AMBER);
            paint.setTextSize(dp(20));
            paint.setFakeBoldText(true);
            paint.setTextAlign(Paint.Align.LEFT);
            canvas.drawText("নোটবুক (Notes App)", dp(20), y + dp(20), paint);

            // + Add Note button
            RectF addBtn = new RectF(w - dp(120), y, w - dp(20), y + dp(32));
            paint.setStyle(Paint.Style.FILL);
            paint.setColor(COLOR_AMBER);
            canvas.drawRoundRect(addBtn, dp(8), dp(8), paint);
            paint.setColor(0xFF0A0E17);
            paint.setTextSize(dp(12));
            paint.setTextAlign(Paint.Align.CENTER);
            canvas.drawText("+ নতুন নোট", addBtn.centerX(), y + dp(21), paint);
            touchAreas.add(new TouchArea(addBtn, "notes_add_new"));

            y += dp(44);

            for (int i = 0; i < notesList.size(); i++) {
                String[] n = notesList.get(i);
                RectF card = new RectF(dp(20), y, w - dp(20), y + dp(66));
                paint.setStyle(Paint.Style.FILL);
                paint.setColor(getSurfaceColor());
                canvas.drawRoundRect(card, dp(12), dp(12), paint);

                paint.setColor(COLOR_AMBER);
                paint.setTextSize(dp(14));
                paint.setFakeBoldText(true);
                paint.setTextAlign(Paint.Align.LEFT);
                canvas.drawText(n[1], dp(34), y + dp(24), paint);

                paint.setColor(getTextMedColor());
                paint.setTextSize(dp(11));
                paint.setTextAlign(Paint.Align.RIGHT);
                canvas.drawText(n[3] + " • " + n[5], w - dp(34), y + dp(24), paint);

                paint.setColor(getTextHighColor());
                paint.setTextSize(dp(12));
                paint.setFakeBoldText(false);
                paint.setTextAlign(Paint.Align.LEFT);
                String snip = n[2].length() > 36 ? n[2].substring(0, 36) + "..." : n[2];
                canvas.drawText(snip, dp(34), y + dp(48), paint);

                touchAreas.add(new TouchArea(card, "notes_open_" + i));
                y += dp(76);
            }

            if (selectedNoteDetail != null) {
                RectF detailBox = new RectF(dp(20), y, w - dp(20), y + dp(90));
                paint.setColor(0xFF1E293B);
                canvas.drawRoundRect(detailBox, dp(12), dp(12), paint);
                paint.setColor(COLOR_CYAN);
                paint.setTextSize(dp(12));
                paint.setTextAlign(Paint.Align.LEFT);
                canvas.drawText(selectedNoteDetail, dp(34), y + dp(30), paint);
            }
        }

        // ─── Music App (Aligned with music-streaming-nilLang) ───────────────────────

        private void renderMusicScreen(Canvas canvas, int w, int h) {
            float y = dp(52);
            paint.setColor(COLOR_GREEN);
            paint.setTextSize(dp(20));
            paint.setFakeBoldText(true);
            paint.setTextAlign(Paint.Align.LEFT);
            canvas.drawText("অনুরণ মিউজিক (NilMusic)", dp(20), y + dp(20), paint);

            y += dp(36);

            // Vinyl Album Card
            RectF albumCard = new RectF(dp(20), y, w - dp(20), y + dp(140));
            paint.setStyle(Paint.Style.FILL);
            paint.setColor(getSurfaceColor());
            canvas.drawRoundRect(albumCard, dp(16), dp(16), paint);
            paint.setStyle(Paint.Style.STROKE);
            paint.setColor(getBorderColor());
            canvas.drawRoundRect(albumCard, dp(16), dp(16), paint);

            // Vinyl Record Disc
            paint.setStyle(Paint.Style.FILL);
            paint.setColor(0xFF0F172A);
            canvas.drawCircle(dp(74), y + dp(70), dp(40), paint);
            paint.setColor(COLOR_GREEN);
            canvas.drawCircle(dp(74), y + dp(70), dp(12), paint);

            String[] track = musicTracks.get(musicCurrentTrack);
            paint.setColor(getTextHighColor());
            paint.setTextSize(dp(15));
            paint.setFakeBoldText(true);
            paint.setTextAlign(Paint.Align.LEFT);
            canvas.drawText(track[0], dp(130), y + dp(46), paint);

            paint.setColor(COLOR_CYAN);
            paint.setTextSize(dp(12));
            paint.setFakeBoldText(false);
            canvas.drawText(track[1], dp(130), y + dp(68), paint);

            // Scrubber Bar
            paint.setColor(darkMode ? 0xFF334155 : 0xFFCBD5E1);
            canvas.drawRoundRect(new RectF(dp(130), y + dp(86), w - dp(36), y + dp(90)), dp(2), dp(2), paint);
            paint.setColor(COLOR_GREEN);
            canvas.drawRoundRect(new RectF(dp(130), y + dp(86), dp(130) + dp(60), y + dp(90)), dp(2), dp(2), paint);

            // Controls: ⏮ ▶/⏸ ⏭
            float btnCenterY = y + dp(116);
            paint.setTextSize(dp(20));
            paint.setTextAlign(Paint.Align.CENTER);
            paint.setColor(COLOR_CYAN);
            canvas.drawText("⏮", dp(160), btnCenterY, paint);
            touchAreas.add(new TouchArea(new RectF(dp(140), btnCenterY - dp(16), dp(180), btnCenterY + dp(16)), "music_prev"));

            paint.setColor(COLOR_GREEN);
            canvas.drawText(musicPlaying ? "⏸" : "▶", dp(210), btnCenterY, paint);
            touchAreas.add(new TouchArea(new RectF(dp(190), btnCenterY - dp(16), dp(230), btnCenterY + dp(16)), "music_toggle_play"));

            paint.setColor(COLOR_CYAN);
            canvas.drawText("⏭", dp(260), btnCenterY, paint);
            touchAreas.add(new TouchArea(new RectF(dp(240), btnCenterY - dp(16), dp(280), btnCenterY + dp(16)), "music_next"));

            y += dp(154);

            // Playlist
            paint.setColor(getTextMedColor());
            paint.setTextSize(dp(13));
            paint.setTextAlign(Paint.Align.LEFT);
            canvas.drawText("প্লে-লিস্ট (Onuron Playlist):", dp(20), y + dp(14), paint);

            y += dp(24);

            for (int i = 0; i < musicTracks.size(); i++) {
                String[] t = musicTracks.get(i);
                RectF card = new RectF(dp(20), y, w - dp(20), y + dp(46));
                paint.setStyle(Paint.Style.FILL);
                paint.setColor(i == musicCurrentTrack ? (darkMode ? 0xFF064E3B : 0xFFDCFCE7) : getSurfaceColor());
                canvas.drawRoundRect(card, dp(10), dp(10), paint);

                paint.setColor(i == musicCurrentTrack ? COLOR_GREEN : getTextHighColor());
                paint.setTextSize(dp(13));
                paint.setFakeBoldText(i == musicCurrentTrack);
                paint.setTextAlign(Paint.Align.LEFT);
                canvas.drawText((i + 1) + ". " + t[0], dp(34), y + dp(28), paint);

                paint.setColor(getTextMedColor());
                paint.setTextSize(dp(11.5f));
                paint.setTextAlign(Paint.Align.RIGHT);
                canvas.drawText(t[2], w - dp(34), y + dp(28), paint);

                touchAreas.add(new TouchArea(card, "music_select_" + i));
                y += dp(52);
            }
        }

        // ─── About Screen ───────────────────────────────────────────────────────────

        private void renderAboutScreen(Canvas canvas, int w, int h) {
            float y = dp(52);
            paint.setColor(COLOR_CYAN);
            paint.setTextSize(dp(20));
            paint.setFakeBoldText(true);
            paint.setTextAlign(Paint.Align.LEFT);
            canvas.drawText("Onuron OS সম্পর্কে (About)", dp(20), y + dp(20), paint);

            y += dp(36);
            RectF card = new RectF(dp(20), y, w - dp(20), y + dp(210));
            paint.setStyle(Paint.Style.FILL);
            paint.setColor(getSurfaceColor());
            canvas.drawRoundRect(card, dp(14), dp(14), paint);

            paint.setStyle(Paint.Style.STROKE);
            paint.setColor(getBorderColor());
            canvas.drawRoundRect(card, dp(14), dp(14), paint);

            paint.setStyle(Paint.Style.FILL);
            paint.setTextSize(dp(12.5f));
            paint.setFakeBoldText(false);
            paint.setTextAlign(Paint.Align.LEFT);

            float ty = y + dp(30);
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
                paint.setColor(l.startsWith("📘") ? COLOR_CYAN : getTextMedColor());
                canvas.drawText(l, dp(34), ty, paint);
                ty += dp(24);
            }
        }

        // ─── Bottom Navigation Bar ──────────────────────────────────────────────────

        private void renderBottomNav(Canvas canvas, int w, int h) {
            int navH = dp(52);
            float navY = h - navH;

            paint.setStyle(Paint.Style.FILL);
            paint.setColor(darkMode ? 0xFF0C121E : 0xFFFFFFFF);
            canvas.drawRect(0, navY, w, h, paint);

            paint.setColor(getBorderColor());
            canvas.drawRect(0, navY, w, navY + 1, paint);

            float btnW = (w - dp(32)) / 3f;
            float btnH = dp(38);
            float btnY = navY + dp(7);

            // Back button
            RectF backRect = new RectF(dp(8), btnY, dp(8) + btnW, btnY + btnH);
            paint.setColor(getSurfaceColor());
            canvas.drawRoundRect(backRect, dp(10), dp(10), paint);
            paint.setColor(getTextMedColor());
            paint.setTextSize(dp(13));
            paint.setTextAlign(Paint.Align.CENTER);
            canvas.drawText("◀ ব্যাক", backRect.centerX(), btnY + dp(24), paint);
            touchAreas.add(new TouchArea(backRect, "nav_back"));

            // Home button
            RectF homeRect = new RectF(dp(16) + btnW, btnY, dp(16) + 2 * btnW, btnY + btnH);
            paint.setColor(screen == Screen.HOME ? (darkMode ? 0xFF0C2E55 : 0xFFE0F2FE) : getSurfaceColor());
            canvas.drawRoundRect(homeRect, dp(10), dp(10), paint);
            paint.setColor(COLOR_CYAN);
            canvas.drawText("⌂ হোম", homeRect.centerX(), btnY + dp(24), paint);
            touchAreas.add(new TouchArea(homeRect, "nav_home"));

            // Settings button
            RectF setRect = new RectF(dp(24) + 2 * btnW, btnY, w - dp(8), btnY + btnH);
            paint.setColor(getSurfaceColor());
            canvas.drawRoundRect(setRect, dp(10), dp(10), paint);
            paint.setColor(COLOR_PURPLE);
            canvas.drawText("⚙️ সেটিংস", setRect.centerX(), btnY + dp(24), paint);
            touchAreas.add(new TouchArea(setRect, "nav_settings"));
        }

        @Override
        protected void onSizeChanged(int w, int h, int oldw, int oldh) {
            super.onSizeChanged(w, h, oldw, oldh);
            NativeBridge.onSurfaceChanged(null, w, h);
        }

        @Override
        public boolean onTouchEvent(MotionEvent event) {
            int action = event.getActionMasked();
            int pointerId = event.getPointerId(event.getActionIndex());
            float x = event.getX();
            float y = event.getY();
            float pressure = event.getPressure();
            NativeBridge.onHostTouchEvent(action, pointerId, x, y, pressure);

            if (event.getAction() == MotionEvent.ACTION_DOWN) {
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
                    activity.setMessageInputVisible(false);
                    activity.hideBrowser();
                    inActiveChat = false;
                    break;
                case "nav_settings":
                case "app_settings":
                    screen = Screen.SETTINGS;
                    activity.setTerminalInputVisible(false);
                    activity.setMessageInputVisible(false);
                    activity.hideBrowser();
                    break;
                case "app_phone":
                    screen = Screen.PHONE;
                    activity.setTerminalInputVisible(false);
                    activity.setMessageInputVisible(false);
                    activity.hideBrowser();
                    break;
                case "app_browser":
                    activity.showBrowser("https://duckduckgo.com");
                    break;
                case "app_messages":
                    screen = Screen.MESSAGES;
                    activity.setTerminalInputVisible(false);
                    activity.setMessageInputVisible(inActiveChat);
                    activity.hideBrowser();
                    break;
                case "app_files":
                    screen = Screen.FILES;
                    activity.setTerminalInputVisible(false);
                    activity.setMessageInputVisible(false);
                    activity.hideBrowser();
                    break;
                case "app_terminal":
                    screen = Screen.TERMINAL;
                    activity.setTerminalInputVisible(true);
                    activity.setMessageInputVisible(false);
                    activity.hideBrowser();
                    break;
                case "app_camera":
                    activity.triggerCamera();
                    break;
                case "app_calculator":
                    screen = Screen.CALCULATOR;
                    activity.setTerminalInputVisible(false);
                    activity.setMessageInputVisible(false);
                    activity.hideBrowser();
                    break;
                case "app_notes":
                    screen = Screen.NOTES;
                    activity.setTerminalInputVisible(false);
                    activity.setMessageInputVisible(false);
                    activity.hideBrowser();
                    break;
                case "app_music":
                    screen = Screen.MUSIC;
                    activity.setTerminalInputVisible(false);
                    activity.setMessageInputVisible(false);
                    activity.hideBrowser();
                    break;
                case "app_about":
                case "app_softbus":
                    screen = Screen.ABOUT;
                    activity.setTerminalInputVisible(false);
                    activity.setMessageInputVisible(false);
                    activity.hideBrowser();
                    break;
                case "open_keyboard":
                    activity.setTerminalInputVisible(true);
                    break;
                case "toggle_wifi":
                    if (!wifiEnabled) {
                        wifiEnabled = true;
                        wifiSignalLevel = 3;
                    } else if (wifiSignalLevel > 1) {
                        wifiSignalLevel--;
                    } else {
                        wifiEnabled = false;
                    }
                    break;
                case "toggle_cellular":
                    cellularEnabled = !cellularEnabled;
                    break;
                case "cycle_signal":
                    if (cellularEnabled) {
                        cellularSignalLevel = (cellularSignalLevel == 1) ? 4 : (cellularSignalLevel - 1);
                    } else {
                        cellularEnabled = true;
                        cellularSignalLevel = 4;
                    }
                    break;
                case "toggle_battery":
                    isCharging = !isCharging;
                    batteryLevel = isCharging ? 100 : 85;
                    break;
                case "toggle_vibration":
                    vibratorEnabled = !vibratorEnabled;
                    break;
                case "toggle_dark":
                    darkMode = !darkMode;
                    break;
                case "cycle_volume":
                    soundVolume = (soundVolume >= 100) ? 0 : (soundVolume + 25);
                    break;
                case "act_call":
                    if (!dialNumber.isEmpty()) {
                        activity.triggerRealCall(dialNumber);
                    }
                    break;
                case "act_save_contact":
                    if (!dialNumber.isEmpty()) {
                        contacts.add(0, new String[]{"সংরক্ষিত নম্বর (" + contacts.size() + ")", dialNumber});
                        phoneTab = 1; // switch to Contacts tab!
                    }
                    break;
                case "act_del":
                    if (!dialNumber.isEmpty()) {
                        dialNumber = dialNumber.substring(0, dialNumber.length() - 1);
                    }
                    break;
                case "phone_tab_0": phoneTab = 0; break;
                case "phone_tab_1": phoneTab = 1; break;
                case "phone_tab_2": phoneTab = 2; break;
                case "file_go_up":
                    File parent = new File(currentPath).getParentFile();
                    if (parent != null && parent.exists()) {
                        currentPath = parent.getAbsolutePath();
                    }
                    break;
                case "files_new_file":
                    try {
                        File nf = new File(currentPath, "অনুরণ_নোট_" + (System.currentTimeMillis() % 1000) + ".txt");
                        nf.createNewFile();
                        selectedFileInfo = "নতুন ফাইল তৈরি হয়েছে: " + nf.getName();
                    } catch (Exception e) {
                        selectedFileInfo = "ফাইল তৈরিতে ত্রুটি!";
                    }
                    break;
                case "msg_new_chat":
                    activeRecipient = "নতুন প্রেরক (+88017...)";
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
                case "notes_add_new":
                    notesList.add(0, new String[]{"note-" + (notesList.size() + 1), "নতুন খসড়া নোট", "অনুরণ ওএস দ্রুত টাস্ক নোট।", "ব্যক্তিগত", "Normal", "আজ"});
                    break;
                default:
                    if (id.startsWith("key_")) {
                        dialNumber += id.substring(4);
                    } else if (id.startsWith("chip_")) {
                        runCommand(id.substring(5));
                    } else if (id.startsWith("call_contact_")) {
                        int idx = Integer.parseInt(id.substring(13));
                        if (idx < contacts.size()) {
                            activity.triggerRealCall(contacts.get(idx)[1]);
                        }
                    } else if (id.startsWith("redial_log_")) {
                        int idx = Integer.parseInt(id.substring(11));
                        if (idx < callLogs.size()) {
                            activity.triggerRealCall(callLogs.get(idx)[0]);
                        }
                    } else if (id.startsWith("open_chat_")) {
                        int idx = Integer.parseInt(id.substring(10));
                        if (idx < messageThreads.size()) {
                            activeRecipient = messageThreads.get(idx)[0];
                            inActiveChat = true;
                            activity.setMessageInputVisible(true);
                        }
                    } else if (id.startsWith("file_click_")) {
                        int idx = Integer.parseInt(id.substring(11));
                        File[] fList = new File(currentPath).listFiles();
                        if (fList != null && idx < fList.length) {
                            File sel = fList[idx];
                            if (sel.isDirectory()) {
                                currentPath = sel.getAbsolutePath();
                            } else {
                                selectedFileInfo = "ফাইল: " + sel.getName() + " (" + (sel.length() / 1024) + " KB)";
                            }
                        }
                    } else if (id.startsWith("calc_key_")) {
                        handleCalc(id.substring(9));
                    } else if (id.startsWith("music_select_")) {
                        musicCurrentTrack = Integer.parseInt(id.substring(13));
                        musicPlaying = true;
                    } else if (id.startsWith("notes_open_")) {
                        int idx = Integer.parseInt(id.substring(11));
                        if (idx < notesList.size()) {
                            selectedNoteDetail = notesList.get(idx)[1] + ":\n" + notesList.get(idx)[2];
                        }
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
