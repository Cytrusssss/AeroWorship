use aeroworship_core::models::{flag_primary, monitor_id, select_output_monitor, Monitor};
use tauri::{
    webview::DownloadEvent, window::Color, AppHandle, Manager, PhysicalPosition, PhysicalSize,
    Runtime, WebviewUrl, WebviewWindowBuilder, WindowEvent,
};

use crate::services::webview_chrome;

const CONTROL_WINDOW_LABEL: &str = "main";

pub const OUTPUT_WINDOW_LABEL: &str = "output";

const OUTPUT_ENTRY: &str = "output.html";

const OUTPUT_TITLE: &str = "AeroWorship Projector Output";

const OUTPUT_BACKGROUND: Color = Color(0, 0, 0, 255);

pub fn connected_monitors<R: Runtime>(app: &AppHandle<R>) -> Vec<Monitor> {
    let primary_id = app
        .primary_monitor()
        .ok()
        .flatten()
        .map(|monitor| monitor_id(monitor.name().map(String::as_str), position_of(&monitor)));

    let reported = app
        .available_monitors()
        .unwrap_or_default()
        .iter()
        .map(|monitor| {
            Monitor::from_os_report(
                monitor.name().map(String::as_str),
                (monitor.size().width, monitor.size().height),
                position_of(monitor),
                monitor.scale_factor(),
                false,
            )
        })
        .collect();

    flag_primary(reported, primary_id.as_deref())
}

pub fn init<R: Runtime>(app: &AppHandle<R>) {
    let monitors = connected_monitors(app);
    let Some(target) = select_output_monitor(&monitors) else {
        return;
    };
    if let Err(error) = open_output_window(app, target) {
        eprintln!("FR-102: could not open the output window: {error}");
    }
}

fn close_output_with_control_panel<R: Runtime>(app: &AppHandle<R>) {
    let Some(control) = app.get_webview_window(CONTROL_WINDOW_LABEL) else {
        return;
    };
    let app = app.clone();
    control.on_window_event(move |event| {
        if matches!(event, WindowEvent::Destroyed) {
            if let Some(output) = app.get_webview_window(OUTPUT_WINDOW_LABEL) {
                let _ = output.close();
            }
        }
    });
}

fn open_output_window<R: Runtime>(app: &AppHandle<R>, target: &Monitor) -> tauri::Result<()> {
    let window = WebviewWindowBuilder::new(
        app,
        OUTPUT_WINDOW_LABEL,
        WebviewUrl::App(OUTPUT_ENTRY.into()),
    )
    .title(OUTPUT_TITLE)
    .background_color(OUTPUT_BACKGROUND)
    .decorations(false)
    .resizable(false)
    .skip_taskbar(true)
    .focused(false)
    .visible(false)
    .on_download(|_webview, event| {
        if let DownloadEvent::Requested { url, .. } = event {
            eprintln!(
                "FR-105: refused a download from the output window (a `{}:` URL). The rest \
                 of the URL is deliberately omitted: it can carry the slide text.",
                url.scheme()
            );
        }
        false
    })
    .build()?;

    close_output_with_control_panel(app);

    webview_chrome::harden(&window);

    window.set_position(PhysicalPosition::new(target.x, target.y))?;
    window.set_size(PhysicalSize::new(target.width, target.height))?;
    window.set_fullscreen(true)?;
    window.show()?;

    Ok(())
}

fn position_of(monitor: &tauri::Monitor) -> (i32, i32) {
    let position = monitor.position();
    (position.x, position.y)
}
