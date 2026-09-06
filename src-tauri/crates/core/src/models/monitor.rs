use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(test, derive(ts_rs::TS))]
#[cfg_attr(test, ts(export))]
pub struct Monitor {
    pub id: String,
    pub name: String,
    pub width: u32,
    pub height: u32,
    pub x: i32,
    pub y: i32,
    pub scale_factor: f64,
    pub is_primary: bool,
}

impl Monitor {
    pub fn from_os_report(
        name: Option<&str>,
        size: (u32, u32),
        position: (i32, i32),
        scale_factor: f64,
        is_primary: bool,
    ) -> Self {
        let name = reported_name(name);
        let id = monitor_id(name, position);
        Self {
            name: name.map_or_else(|| id.clone(), str::to_owned),
            id,
            width: size.0,
            height: size.1,
            x: position.0,
            y: position.1,
            scale_factor,
            is_primary,
        }
    }
}

pub fn flag_primary(monitors: Vec<Monitor>, primary_id: Option<&str>) -> Vec<Monitor> {
    monitors
        .into_iter()
        .map(|monitor| Monitor {
            is_primary: primary_id == Some(monitor.id.as_str()),
            ..monitor
        })
        .collect()
}

pub fn select_output_monitor(monitors: &[Monitor]) -> Option<&Monitor> {
    if !monitors.iter().any(|monitor| monitor.is_primary) {
        return None;
    }
    monitors
        .iter()
        .filter(|monitor| !monitor.is_primary)
        .min_by_key(|monitor| (monitor.x, monitor.y, monitor.id.as_str()))
}

const SCHEME_OS_NAME: &str = "gdi:";

const SCHEME_POSITION: &str = "pos:";

fn reported_name(name: Option<&str>) -> Option<&str> {
    name.filter(|name| !name.trim().is_empty())
}

pub fn monitor_id(name: Option<&str>, position: (i32, i32)) -> String {
    match reported_name(name) {
        Some(name) => format!("{SCHEME_OS_NAME}{name}"),
        None => format!("{SCHEME_POSITION}{},{}", position.0, position.1),
    }
}
