//! The app ACL manifest, asserted rather than read (SEC-01, ADR-0041, NFR-14).
//!
//! `capabilities/output-window.json` grants the Projector Output window
//! nothing, and `capabilities/main-window.json` grants `allow-list-monitors` to
//! `main` alone. Until this file existed, that was a claim a reader checked by
//! opening three JSON files and trusting `tauri`'s source about what they do.
//! Here it is a behaviour: the same `InvokeRequest`, for the same registered
//! command, is dispatched from `main` and refused from `output`.
//!
//! **Why this can be tested with no GUI at all.** The ACL gate lives in
//! `Webview::on_message` (`tauri` 2.11.5, `webview/mod.rs:1823`/`:1850`) and has
//! no branch on `R: Runtime`; `tauri::test::get_ipc_response` drives that same
//! function. So the `MockRuntime` answers the ACL question exactly as `wry`
//! does, and the parts a mock cannot reproduce — a real projector, a real
//! webview — are not parts this file asks about.
//!
//! **The trap, and why two of the tests below exist only to spring it.**
//! `tauri::test::mock_context(noop_assets())` installs `Resolved::default()`,
//! so `RuntimeAuthority::has_app_manifest()` is `false` and the gate is never
//! entered for an app-defined command: with that context there is no ACL at
//! all, both windows are dispatched, and a test written against it is green
//! while measuring nothing. That is why the app under test is built from the
//! real `generate_context!()`, which carries the resolved manifest
//! `tauri_build::build()` produced from `permissions/` and `capabilities/`.
//! [`no_acl_is_loaded_from_the_mock_context_so_the_same_call_is_dispatched`] and
//! [`the_refusal_comes_from_the_capabilities_this_repository_ships`] are the
//! positive controls: the first pins the trap open, by showing that the very
//! same request from the very same label is *dispatched* once the manifest is
//! taken away, so a refusal cannot be blamed on the harness, the invoke key,
//! the URL or the label; the second shows the refusal quotes `main-window` and
//! `allow-list-monitors`, which only a manifest built from this repository's
//! files can name.
//!
//! **What "allowed" looks like here, and why it is not `Ok`.** `list_monitors`
//! calls `AppHandle::primary_monitor`, and `MockRuntime` answers that with
//! `unimplemented!()` (`tauri` 2.11.5, `test/mock_runtime.rs:245`); the
//! `available_monitors` it calls next is `unimplemented!()` as well, at `:253`.
//! Neither returns an empty `Vec`. The arm that does return `Ok(Vec::new())`,
//! at `:797`, belongs to `WindowDispatch` and is reached from a `Window`, which
//! is not the receiver `connected_monitors` holds — `src/services/display.rs`
//! says exactly this, and says it correctly; do not "fix" it back. So a call
//! the ACL lets through reaches the command body and dies there, and a call
//! it refuses never gets that far. Reaching the body *is* the whole of what
//! "allowed" means to SEC-01: what `list_monitors` answers on a machine with
//! displays is FR-101's question, and
//! [`aeroworship_core::models::flag_primary`] is where it is asked.
//!
//! **What is deliberately not asserted.** The literal
//! `"Command list_monitors not allowed by ACL"` is the
//! `#[cfg(not(debug_assertions))]` arm of that same rejection; under
//! `cargo test` the message is `resolve_access_message`, a different sentence.
//! Both are asserted for what they always contain — the command name — and the
//! richer one is examined only where it exists, so `npm run test:perf:shell`
//! (a `--release` run) does not go red on a message that profile never
//! produces.
//!
//! **If this whole file dies at load with `STATUS_ENTRYPOINT_NOT_FOUND`
//! (exit 127), read `src-tauri/build.rs`, not this file.** Nothing here is
//! wrong in that case: the process never reaches `main`. A test binary of
//! this package needs the Windows application manifest that
//! `link_app_manifest_into_test_binaries()` links into it, because
//! `common-controls-v6` (ADR-0013) makes `muda` import `TaskDialogIndirect`,
//! which only the side-by-side ComCtl32 v6 assembly exports. The symptom is
//! spelled out here so that grepping for it finds both files, since the
//! failing gate names this one and the cause is in the other.
//!
//! This file lives under `src-tauri/tests/` rather than the repository's
//! `tests/integration/` because Cargo compiles an integration test only from
//! the `tests/` directory of the package it belongs to — the same reason
//! `crates/core/tests/` is where the core suite is. PRD §6.13's
//! `tests/integration/` is the tree this is the shell-crate half of.

use std::panic::{catch_unwind, AssertUnwindSafe};

use aeroworship::commands;
use tauri::ipc::CallbackFn;
use tauri::test::{get_ipc_response, mock_builder, mock_context, noop_assets, MockRuntime};
use tauri::webview::InvokeRequest;
use tauri::{App, WebviewWindowBuilder};

/// The label of the window that is granted nothing, spelled as
/// `capabilities/output-window.json` spells it.
const OUTPUT: &str = "output";
/// The label of the Control Panel, spelled as `capabilities/main-window.json`
/// and `tauri.conf.json` spell it.
const MAIN: &str = "main";
/// The one app-defined command this application registers today (FR-101).
const COMMAND: &str = "list_monitors";

/// What became of one invoke, as far as SEC-01 is concerned.
#[derive(Debug)]
enum Outcome {
    /// The ACL let the call through and the command body ran, then panicked
    /// inside `MockRuntime`. The payload is carried so a test can show the
    /// panic came from the runtime the command called, not from the IPC layer.
    Dispatched(String),
    /// The ACL refused the call before any dispatch, with this message.
    Refused(String),
    /// The command answered. Impossible today, because `MockRuntime` cannot
    /// enumerate monitors; kept so that a future `tauri` which can does not
    /// quietly turn "was allowed" into "was anything at all".
    Answered,
}

/// The request a webview sends when the frontend calls `invoke("list_monitors")`.
///
/// Everything in it except the command name is what `tauri`'s own test module
/// documents; none of it is what decides the answer, which is the point of
/// building it in one place and reusing it for every label below.
fn invoke_list_monitors() -> InvokeRequest {
    InvokeRequest {
        cmd: COMMAND.into(),
        callback: CallbackFn(0),
        error: CallbackFn(1),
        url: "http://tauri.localhost".parse().unwrap(),
        body: Default::default(),
        headers: Default::default(),
        invoke_key: tauri::test::INVOKE_KEY.to_string(),
    }
}

/// The application as it ships: the real `generate_context!()`, so the resolved
/// ACL is the one `tauri_build::build()` wrote from `src-tauri/permissions/`
/// and `src-tauri/capabilities/`.
///
/// `Builder::build` does not run `setup`, so the windows declared in
/// `tauri.conf.json` are not created here and every test names its own labels.
fn app_as_shipped() -> App<MockRuntime> {
    mock_builder()
        .invoke_handler(tauri::generate_handler![commands::display::list_monitors])
        .build(tauri::generate_context!())
        .expect("the shipped tauri.conf.json and capabilities should build an app")
}

/// The same application with **no** ACL manifest — the trap, built on purpose.
///
/// Used only by the positive control. If a future edit swapped
/// [`app_as_shipped`] for this, every "is refused" test in this file would flip
/// to green-for-the-wrong-reason; the control is what makes that flip visible,
/// because it asserts the *opposite* outcome from the same call.
fn app_with_no_acl_manifest() -> App<MockRuntime> {
    mock_builder()
        .invoke_handler(tauri::generate_handler![commands::display::list_monitors])
        .build(mock_context(noop_assets()))
        .expect("the mock context should build an app")
}

/// Keeps `MockRuntime`'s `unimplemented!()` off the test log without hiding
/// anything else.
///
/// A dispatched call panics by design here (see the module docs), and the
/// default hook would print a page of "not implemented" above a passing run.
/// The hook installed instead forwards every panic that is *not* from
/// `tauri`'s `test/mock_runtime.rs` to the original one, so an assertion
/// failure in this file still prints in full — a blanket silencer would make
/// every future failure here unreadable, which is a worse trade than log noise.
fn quieten_expected_mock_runtime_panics() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        let previous = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            let from_mock_runtime = info
                .location()
                .is_some_and(|location| location.file().ends_with("mock_runtime.rs"));
            if !from_mock_runtime {
                previous(info);
            }
        }));
    });
}

/// Invokes `list_monitors` from a webview window with `label`, as that window.
///
/// The error is flattened to a `String` so this file never names `serde_json`,
/// which the shell crate deliberately does not declare.
fn call_from(app: &App<MockRuntime>, label: &str) -> Outcome {
    quieten_expected_mock_runtime_panics();

    let webview = WebviewWindowBuilder::new(app, label, Default::default())
        .build()
        .expect("the mock runtime should create a webview window");
    let request = invoke_list_monitors();

    match catch_unwind(AssertUnwindSafe(|| get_ipc_response(&webview, request))) {
        Ok(Ok(_)) => Outcome::Answered,
        Ok(Err(error)) => Outcome::Refused(error.to_string()),
        Err(payload) => Outcome::Dispatched(describe_panic(&payload)),
    }
}

/// The message of a caught panic, for use in a diagnostic.
fn describe_panic(payload: &Box<dyn std::any::Any + Send>) -> String {
    if let Some(message) = payload.downcast_ref::<&str>() {
        (*message).to_string()
    } else if let Some(message) = payload.downcast_ref::<String>() {
        message.clone()
    } else {
        "a panic with a payload this test cannot read".to_string()
    }
}

/// A `serde_json` string renders with its inner quotes escaped; the manifest
/// spells window labels with plain ones. Unescaping keeps the needles in the
/// assertions readable as the manifest writes them.
fn unescaped(message: &str) -> String {
    message.replace("\\\"", "\"")
}

#[test]
fn the_main_window_may_call_the_command_its_capability_grants() {
    let app = app_as_shipped();

    match call_from(&app, MAIN) {
        Outcome::Refused(error) => panic!(
            "`{COMMAND}` is granted to `{MAIN}` by capabilities/main-window.json, so the Control \
             Panel must still be able to call it. The ACL is now refusing the window it was \
             written for, which means the application is dead on start-up rather than merely \
             hardened: {error}"
        ),
        Outcome::Dispatched(panic_message) => assert!(
            panic_message.contains("not implemented"),
            "the call reached the command body, which is what `granted` means — but it died of \
             something other than MockRuntime's unimplemented monitor enumeration, so this test \
             is no longer watching what it claims: {panic_message}"
        ),
        Outcome::Answered => {}
    }
}

#[test]
fn the_output_window_may_not_call_it() {
    let app = app_as_shipped();

    let outcome = call_from(&app, OUTPUT);

    let Outcome::Refused(error) = &outcome else {
        panic!(
            "the `{OUTPUT}` window is named by no capability that grants an app-defined command, \
             so the ACL must refuse `{COMMAND}` before it is dispatched. It was not refused: \
             {outcome:?}"
        )
    };
    assert!(
        error.contains(COMMAND),
        "the refusal should name the command that was refused; got {error}"
    );
}

/// The positive control for "the ACL is switched on in this harness".
///
/// Same command, same request, same label, same builder — only the context
/// differs, and the context is where the resolved ACL lives. A green run here
/// together with a green [`the_output_window_may_not_call_it`] is the pair that
/// says the refusal above came from the manifest and from nothing else: no
/// invoke key, URL, window label or missing registration can be responsible,
/// because all of them are identical between the two and this one gets through.
#[test]
fn no_acl_is_loaded_from_the_mock_context_so_the_same_call_is_dispatched() {
    let app = app_with_no_acl_manifest();

    let outcome = call_from(&app, OUTPUT);

    assert!(
        !matches!(outcome, Outcome::Refused(_)),
        "tauri::test::mock_context installs Resolved::default(), so has_app_manifest() is false \
         and app-defined commands are not gated at all. This call being refused means the \
         harness refuses it for some reason other than the ACL — and the refusal asserted by \
         `the_output_window_may_not_call_it` no longer proves what it claims: {outcome:?}"
    );
}

/// The positive control for "the ACL that refused it is *this repository's* ACL".
///
/// Under `debug_assertions` the rejection is `resolve_access_message`, which
/// prints the windows the command *is* allowed on and the capability and
/// permission that allow it. Those three strings — `"main"`, `main-window`,
/// `allow-list-monitors` — cannot come from anywhere but a manifest built from
/// `src-tauri/capabilities/main-window.json` and
/// `src-tauri/permissions/list_monitors.json`. A `Resolved::default()` would
/// have nothing to print and would say "command not allowed on any
/// window/webview/URL context" instead.
///
/// `#[cfg(debug_assertions)]` because the release arm of the same branch emits
/// the fixed `"Command {} not allowed by ACL"` string instead; asserting the
/// rich message unconditionally would make `npm run test:perf:shell` red on a
/// sentence that profile never produces. The command name itself is asserted in
/// both profiles, above.
#[test]
#[cfg(debug_assertions)]
fn the_refusal_comes_from_the_capabilities_this_repository_ships() {
    let app = app_as_shipped();

    let Outcome::Refused(error) = call_from(&app, OUTPUT) else {
        panic!("the `{OUTPUT}` window must be refused before this control can say anything")
    };
    let error = unescaped(&error);

    for expected in ["main-window", "allow-list-monitors", "windows: \"main\""] {
        assert!(
            error.contains(expected),
            "the rejection should quote {expected:?}, which only a manifest resolved from this \
             repository's permissions/ and capabilities/ can name. A refusal without it would \
             mean the ACL is empty rather than ours; got {error}"
        );
    }
    assert!(
        error.contains("not allowed on window \"output\""),
        "the rejection should name the window it refused, so a reader of a failing gate knows \
         which one lost its grant; got {error}"
    );
}

/// The rule is "granted to the labels a capability names", not "everything
/// except `output`".
///
/// A label nothing in `capabilities/` mentions is refused just as the output
/// window is, which is what makes the manifest fail-closed: a window added by a
/// later item is powerless until someone writes the grant down (ADR-0013 —
/// `dynamic-acl` is off, so there is no runtime way to add one).
#[test]
fn a_window_no_capability_names_is_refused_as_well() {
    let app = app_as_shipped();

    let outcome = call_from(&app, "preview");

    let Outcome::Refused(error) = &outcome else {
        panic!(
            "a label no capability names must be refused; the ACL is an allow-list, not a \
             deny-list. Got {outcome:?}"
        )
    };
    assert!(
        error.contains(COMMAND),
        "the refusal should name the command that was refused; got {error}"
    );
}
