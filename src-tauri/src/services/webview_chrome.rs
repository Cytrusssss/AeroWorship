use tauri::{Runtime, WebviewWindow};

pub fn harden<R: Runtime>(window: &WebviewWindow<R>) {
    let label = window.label().to_owned();
    let label_for_closure = label.clone();

    if let Err(error) = window.with_webview(move |webview| apply(&label_for_closure, webview)) {
        eprintln!(
            "FR-105: window `{label}` kept WebView2's default context menu and accelerator keys: \
             the platform webview could not be reached ({error})."
        );
    }
}

#[cfg(windows)]
fn apply(label: &str, webview: tauri::webview::PlatformWebview) {
    use webview2_com::Microsoft::Web::WebView2::Win32::ICoreWebView2Settings3;
    use windows_core::Interface;

    let settings = unsafe {
        webview
            .controller()
            .CoreWebView2()
            .and_then(|core| core.Settings())
    };
    let settings = match settings {
        Ok(settings) => settings,
        Err(error) => {
            eprintln!(
                "FR-105: window `{label}` kept WebView2's default chrome: its settings object \
                 could not be read ({error})."
            );
            return;
        }
    };

    if let Err(error) = unsafe { settings.SetAreDefaultContextMenusEnabled(false) } {
        eprintln!(
            "FR-105: window `{label}` still raises the WebView2 context menu on right click \
             ({error})."
        );
    }

    if cfg!(debug_assertions) {
        return;
    }

    match settings.cast::<ICoreWebView2Settings3>() {
        Ok(settings3) => {
            if let Err(error) = unsafe { settings3.SetAreBrowserAcceleratorKeysEnabled(false) } {
                eprintln!(
                    "FR-105: window `{label}` still answers Ctrl-S and Ctrl-P with a native \
                     dialog ({error})."
                );
            }
        }
        Err(error) => {
            eprintln!(
                "FR-105: window `{label}` still answers Ctrl-S and Ctrl-P with a native dialog: \
                 this WebView2 runtime does not implement ICoreWebView2Settings3 ({error})."
            );
        }
    }
}

#[cfg(not(windows))]
fn apply(label: &str, _webview: tauri::webview::PlatformWebview) {
    eprintln!(
        "FR-105: window `{label}` keeps its default context menu and accelerator keys: this is \
         not Windows, and both are WebView2 settings (NFR-17)."
    );
}
