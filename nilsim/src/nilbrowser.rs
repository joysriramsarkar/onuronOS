// ───NilOS WebView2 (Evergreen Chromium) Embedded Browser & Per-App Sandboxed Runtime
// Uses Microsoft WebView2 COM API with isolated User Data Folders per package
#![allow(dead_code, unused_variables, unused_imports)]

#[cfg(target_os = "windows")]
pub mod chromium {
    use std::path::Path;
    use std::sync::mpsc;

    use webview2_com::{
        wait_with_pump, CoreWebView2EnvironmentOptions,
        CreateCoreWebView2ControllerCompletedHandler,
        CreateCoreWebView2EnvironmentCompletedHandler,
        Microsoft::Web::WebView2::Win32::{
            CreateCoreWebView2EnvironmentWithOptions, ICoreWebView2, ICoreWebView2Controller,
            ICoreWebView2Environment, ICoreWebView2EnvironmentOptions,
        },
    };
    use windows::{
        core::PWSTR,
        Win32::{
            Foundation::{E_POINTER, HWND, RECT},
            System::Com::{CoInitializeEx, COINIT_APARTMENTTHREADED},
        },
    };

    pub struct NilBrowser {
        pub controller: ICoreWebView2Controller,
        pub webview: ICoreWebView2,
        pub is_visible: bool,
    }

    unsafe impl Send for NilBrowser {}
    unsafe impl Sync for NilBrowser {}

    pub fn create_embedded_browser(
        parent_hwnd: isize,
        x: i32,
        y: i32,
        w: i32,
        h: i32,
        url: &str,
    ) -> windows::core::Result<NilBrowser> {
        let default_dir = std::env::temp_dir()
            .join("onuron_webview_sandboxes")
            .join("org.onuron.browser");
        create_isolated_webview(parent_hwnd, &default_dir, x, y, w, h, url)
    }

    pub fn create_isolated_webview(
        parent_hwnd: isize,
        user_data_dir: &Path,
        x: i32,
        y: i32,
        w: i32,
        h: i32,
        url: &str,
    ) -> windows::core::Result<NilBrowser> {
        unsafe {
            let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        }
        let _ = std::fs::create_dir_all(user_data_dir);

        let parent = HWND(parent_hwnd as *mut _);
        let url_str = url.to_string();

        // Create WebView2 Environment with isolated user_data_dir and Android Mobile User-Agent
        let env = create_environment(Some(user_data_dir))?;

        // Create Controller embedded into parent HWND
        let controller = create_controller(parent, &env)?;

        // Set bounds: position at (x, y, w, h) inside parent window and apply mobile zoom factor
        let bounds = RECT {
            left: x,
            top: y,
            right: x + w,
            bottom: y + h,
        };
        unsafe {
            controller.SetBounds(bounds)?;
            let _ = controller.SetZoomFactor(0.85);
            controller.SetIsVisible(true)?;
        }

        // Get ICoreWebView2, inject responsive mobile viewport + touch/mobile media overrides + target=_blank trap, and navigate
        let webview = unsafe { controller.CoreWebView2()? };
        let is_whatsapp = user_data_dir.to_string_lossy().contains("whatsapp");
        let init_js = if is_whatsapp {
            "(() => { \
                try { \
                    Object.defineProperty(navigator, 'userAgent', { get: () => 'Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/130.0.0.0 Safari/537.36' }); \
                    Object.defineProperty(navigator, 'platform', { get: () => 'Linux x86_64' }); \
                    window.open = function(url) { if (url) { window.location.href = url; } return window; }; \
                } catch (_) {} \
            })();"
        } else {
            "(() => { \
                try { \
                    Object.defineProperty(navigator, 'userAgent', { get: () => 'Mozilla/5.0 (Linux; Android 14; Pixel 8) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/130.0.0.0 Mobile Safari/537.36' }); \
                    Object.defineProperty(navigator, 'platform', { get: () => 'Linux armv8l' }); \
                    Object.defineProperty(navigator, 'maxTouchPoints', { get: () => 5 }); \
                    window.open = function(url) { if (url) { window.location.href = url; } return window; }; \
                } catch (_) {} \
                document.addEventListener('click', (e) => { \
                    const a = e.target && e.target.closest ? e.target.closest('a[target=\"_blank\"]') : null; \
                    if (a && a.href) { a.removeAttribute('target'); } \
                }, true); \
                document.addEventListener('DOMContentLoaded', () => { \
                    let vp = document.querySelector('meta[name=\"viewport\"]'); \
                    if (!vp) { \
                        vp = document.createElement('meta'); \
                        vp.name = 'viewport'; \
                        vp.content = 'width=device-width, initial-scale=1.0, maximum-scale=1.0, user-scalable=no'; \
                        if (document.head) document.head.appendChild(vp); \
                    } \
                    const s = document.createElement('style'); \
                    s.textContent = 'html, body { max-width: 100vw !important; overflow-x: hidden !important; box-sizing: border-box !important; } img, video, canvas, iframe { max-width: 100% !important; }'; \
                    if (document.head) document.head.appendChild(s); \
                }); \
            })();"
        };
        if is_whatsapp {
            unsafe {
                let _ = controller.SetZoomFactor(0.62);
            }
        }
        let mut js_wide: Vec<u16> = init_js.encode_utf16().chain(std::iter::once(0)).collect();
        unsafe {
            let _ = webview.AddScriptToExecuteOnDocumentCreated(PWSTR(js_wide.as_mut_ptr()), None);
            if let Some(html) = url_str.strip_prefix("html:") {
                let mut html_wide: Vec<u16> =
                    html.encode_utf16().chain(std::iter::once(0)).collect();
                webview.NavigateToString(PWSTR(html_wide.as_mut_ptr()))?;
            } else {
                let mut url_wide: Vec<u16> =
                    url_str.encode_utf16().chain(std::iter::once(0)).collect();
                webview.Navigate(PWSTR(url_wide.as_mut_ptr()))?;
            }
        }

        Ok(NilBrowser {
            controller,
            webview,
            is_visible: true,
        })
    }

    pub fn set_bounds(
        browser: &NilBrowser,
        x: i32,
        y: i32,
        w: i32,
        h: i32,
    ) -> windows::core::Result<()> {
        let bounds = RECT {
            left: x,
            top: y,
            right: x + w,
            bottom: y + h,
        };
        unsafe {
            browser.controller.SetBounds(bounds)?;
        }
        Ok(())
    }

    pub fn set_visible(browser: &mut NilBrowser, visible: bool) -> windows::core::Result<()> {
        if browser.is_visible != visible {
            browser.is_visible = visible;
            unsafe {
                browser.controller.SetIsVisible(visible)?;
            }
        }
        Ok(())
    }

    pub fn navigate_to(browser: &NilBrowser, url: &str) -> windows::core::Result<()> {
        unsafe {
            if let Some(html) = url.strip_prefix("html:") {
                let mut html_wide: Vec<u16> =
                    html.encode_utf16().chain(std::iter::once(0)).collect();
                browser
                    .webview
                    .NavigateToString(PWSTR(html_wide.as_mut_ptr()))?;
            } else {
                let mut url_wide: Vec<u16> = url.encode_utf16().chain(std::iter::once(0)).collect();
                browser.webview.Navigate(PWSTR(url_wide.as_mut_ptr()))?;
            }
        }
        Ok(())
    }

    pub fn get_document_title(browser: &NilBrowser) -> Option<String> {
        unsafe {
            let mut title_pwstr = PWSTR::null();
            if browser.webview.DocumentTitle(&mut title_pwstr).is_ok() && !title_pwstr.is_null() {
                let mut len = 0usize;
                while *title_pwstr.0.add(len) != 0 {
                    len += 1;
                }
                let slice = std::slice::from_raw_parts(title_pwstr.0, len);
                let s = String::from_utf16_lossy(slice);
                windows::Win32::System::Com::CoTaskMemFree(Some(title_pwstr.0 as *const _));
                let trimmed = s.trim().to_string();
                if !trimmed.is_empty() {
                    return Some(trimmed);
                }
            }
        }
        None
    }

    pub fn reload(browser: &NilBrowser) -> windows::core::Result<()> {
        unsafe {
            browser.webview.Reload()?;
        }
        Ok(())
    }

    pub fn go_back(browser: &NilBrowser) -> windows::core::Result<()> {
        unsafe {
            browser.webview.GoBack()?;
        }
        Ok(())
    }

    pub fn go_forward(browser: &NilBrowser) -> windows::core::Result<()> {
        unsafe {
            browser.webview.GoForward()?;
        }
        Ok(())
    }

    pub fn eval_js(browser: &NilBrowser, script: &str) -> windows::core::Result<()> {
        let mut js_wide: Vec<u16> = script.encode_utf16().chain(std::iter::once(0)).collect();
        unsafe {
            browser
                .webview
                .ExecuteScript(PWSTR(js_wide.as_mut_ptr()), None)?;
        }
        Ok(())
    }

    fn create_environment(
        user_data_dir: Option<&Path>,
    ) -> windows::core::Result<ICoreWebView2Environment> {
        let (tx, rx) = mpsc::channel();
        let handler = CreateCoreWebView2EnvironmentCompletedHandler::create(Box::new(
            move |_err, env: Option<ICoreWebView2Environment>| {
                let result = env.ok_or_else(|| windows::core::Error::from(E_POINTER));
                let _ = tx.send(result);
                Ok(())
            },
        ));

        let options = CoreWebView2EnvironmentOptions::default();
        let is_desktop_ua = user_data_dir
            .map(|d| d.to_string_lossy().contains("whatsapp"))
            .unwrap_or(false);
        unsafe {
            if is_desktop_ua {
                options.set_additional_browser_arguments(
                    "--enable-features=OverlayScrollbar,FluentOverlayScrollbars --enable-blink-features=ScrollbarColor --user-agent=\"Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/130.0.0.0 Safari/537.36\"".to_string(),
                );
            } else {
                options.set_additional_browser_arguments(
                    "--enable-features=OverlayScrollbar,FluentOverlayScrollbars --enable-blink-features=ScrollbarColor --user-agent=\"Mozilla/5.0 (Linux; Android 14; Pixel 8) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/130.0.0.0 Mobile Safari/537.36\"".to_string(),
                );
            }
        }
        let env_options: ICoreWebView2EnvironmentOptions = options.into();

        unsafe {
            if let Some(dir) = user_data_dir {
                let mut dir_wide: Vec<u16> = dir
                    .to_string_lossy()
                    .encode_utf16()
                    .chain(std::iter::once(0))
                    .collect();
                CreateCoreWebView2EnvironmentWithOptions(
                    None,
                    PWSTR(dir_wide.as_mut_ptr()),
                    &env_options,
                    &handler,
                )?;
            } else {
                CreateCoreWebView2EnvironmentWithOptions(None, None, &env_options, &handler)?;
            }
        }
        wait_with_pump(rx).map_err(|_| windows::core::Error::from(E_POINTER))?
    }

    fn create_controller(
        hwnd: HWND,
        env: &ICoreWebView2Environment,
    ) -> windows::core::Result<ICoreWebView2Controller> {
        let (tx, rx) = mpsc::channel();
        let handler = CreateCoreWebView2ControllerCompletedHandler::create(Box::new(
            move |_err, ctl: Option<ICoreWebView2Controller>| {
                let result = ctl.ok_or_else(|| windows::core::Error::from(E_POINTER));
                let _ = tx.send(result);
                Ok(())
            },
        ));
        unsafe {
            env.CreateCoreWebView2Controller(hwnd, &handler)?;
        }
        wait_with_pump(rx).map_err(|_| windows::core::Error::from(E_POINTER))?
    }
}

#[cfg(not(target_os = "windows"))]
pub mod chromium {
    pub struct NilBrowser {
        pub is_visible: bool,
    }

    pub fn create_embedded_browser(
        _parent_hwnd: isize,
        _x: i32,
        _y: i32,
        _w: i32,
        _h: i32,
        _url: &str,
    ) -> Result<NilBrowser, String> {
        Ok(NilBrowser { is_visible: true })
    }

    pub fn set_bounds(
        _browser: &NilBrowser,
        _x: i32,
        _y: i32,
        _w: i32,
        _h: i32,
    ) -> Result<(), String> {
        Ok(())
    }

    pub fn set_visible(browser: &mut NilBrowser, visible: bool) -> Result<(), String> {
        browser.is_visible = visible;
        Ok(())
    }

    pub fn navigate_to(_browser: &NilBrowser, _url: &str) -> Result<(), String> {
        Ok(())
    }

    pub fn reload(_browser: &NilBrowser) -> Result<(), String> {
        Ok(())
    }

    pub fn go_back(_browser: &NilBrowser) -> Result<(), String> {
        Ok(())
    }

    pub fn create_isolated_webview(
        _parent_hwnd: isize,
        _user_data_dir: &std::path::Path,
        _x: i32,
        _y: i32,
        _w: i32,
        _h: i32,
        _url: &str,
    ) -> Result<NilBrowser, String> {
        Ok(NilBrowser { is_visible: true })
    }

    pub fn eval_js(_browser: &NilBrowser, _script: &str) -> Result<(), String> {
        Ok(())
    }

    pub fn get_document_title(_browser: &NilBrowser) -> Option<String> {
        None
    }
}
