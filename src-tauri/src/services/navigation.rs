use tauri::plugin::{Builder as PluginBuilder, TauriPlugin};
use tauri::utils::config::{Config, FrontendDist};
use tauri::{Manager, Runtime, Url};

pub const PLUGIN_NAME: &str = "aeroworship-navigation-guard";

pub fn guard<R: Runtime>() -> TauriPlugin<R> {
    PluginBuilder::new(PLUGIN_NAME)
        .on_navigation(|webview, url| {
            let allowed = app_origins(webview.config(), webview.label());
            if is_internal(url, &allowed) {
                return true;
            }
            report_refusal(webview.label(), url, &allowed);
            false
        })
        .build()
}

pub fn is_internal(url: &Url, allowed: &[Url]) -> bool {
    url.host().is_some() && allowed.iter().any(|origin| same_origin(url, origin))
}

pub fn app_origins(config: &Config, label: &str) -> Vec<Url> {
    let mut origins = vec![tauri_protocol_origin(uses_https_scheme(config, label))];

    let configured = if tauri::is_dev() {
        config.build.dev_url.clone()
    } else {
        match config.build.frontend_dist.as_ref() {
            Some(FrontendDist::Url(url)) => Some(url.clone()),
            _ => None,
        }
    };
    if let Some(url) = configured {
        origins.push(url);
    }

    origins
}

fn uses_https_scheme(config: &Config, label: &str) -> bool {
    config
        .app
        .windows
        .iter()
        .find(|window| window.label == label)
        .map(|window| window.use_https_scheme)
        .unwrap_or(false)
}

fn tauri_protocol_origin(https: bool) -> Url {
    let spelling = if cfg!(any(windows, target_os = "android")) {
        if https {
            "https://tauri.localhost"
        } else {
            "http://tauri.localhost"
        }
    } else {
        "tauri://localhost"
    };
    Url::parse(spelling).expect("a literal base URL should parse")
}

fn same_origin(candidate: &Url, origin: &Url) -> bool {
    candidate.scheme() == origin.scheme()
        && candidate.host() == origin.host()
        && candidate.port_or_known_default() == origin.port_or_known_default()
}

fn report_refusal(label: &str, url: &Url, allowed: &[Url]) {
    let allowed = allowed.iter().map(origin_of).collect::<Vec<_>>().join(", ");
    eprintln!(
        "SEC-02: refused a navigation from window `{label}` to {} (this build serves: {allowed}). \
         The path and query are deliberately omitted: they carry the slide text.",
        origin_of(url)
    );
}

fn origin_of(url: &Url) -> String {
    match (url.host_str(), url.port()) {
        (Some(host), Some(port)) => format!("{}://{host}:{port}", url.scheme()),
        (Some(host), None) => format!("{}://{host}", url.scheme()),
        (None, _) => format!("{}: (no host)", url.scheme()),
    }
}
