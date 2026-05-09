use std::fs::OpenOptions;
use std::io::Write;
use std::panic;

use crate::config::app::AppPaths;

pub fn install_panic_hook() {
    panic::set_hook(Box::new(|info| {
        let payload = if let Some(message) = info.payload().downcast_ref::<&str>() {
            (*message).to_string()
        } else if let Some(message) = info.payload().downcast_ref::<String>() {
            message.clone()
        } else {
            "unknown panic payload".to_string()
        };

        let location = info
            .location()
            .map(|location| format!("{}:{}", location.file(), location.line()))
            .unwrap_or_else(|| "unknown location".to_string());

        append_launch_log(&format!("panic at {location}: {payload}"));
    }));
}

pub fn append_launch_log(message: &str) {
    let Ok(paths) = AppPaths::resolve() else {
        return;
    };

    let log_path = paths.app_dir().join("launch.log");
    let Ok(mut file) = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)
    else {
        return;
    };

    let _ = writeln!(file, "{message}");
}
