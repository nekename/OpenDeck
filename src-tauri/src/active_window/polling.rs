use std::time::Duration;

use active_win_pos_rs::get_active_window;
use tauri::Manager;
use tokio::sync::mpsc;

/// Interval while profiles are configured, so switching stays responsive.
const ACTIVE_INTERVAL: Duration = Duration::from_millis(250);
/// Interval while only the UI needs the list of running applications.
const UI_ONLY_INTERVAL: Duration = Duration::from_secs(2);
/// Interval while nothing needs the active window.
const IDLE_INTERVAL: Duration = Duration::from_secs(1);

pub async fn run(tx: mpsc::Sender<String>) {
	let app_handle = crate::APP_HANDLE.get().unwrap();
	let mut previous: Option<String> = None;
	loop {
		let has_profiles = !crate::application_watcher::APPLICATION_PROFILES.read().await.value.is_empty();
		let window_visible = app_handle.get_webview_window("main").and_then(|w| w.is_visible().ok()).unwrap_or(false);

		// Each call can be expensive (on KDE Wayland it opens D-Bus connections and loads a KWin script),
		// so skip it entirely when neither profile switching nor the UI needs the result.
		if !has_profiles && !window_visible {
			// Forget the last value so the current application is re-evaluated once polling resumes.
			previous = None;
			tokio::time::sleep(IDLE_INTERVAL).await;
			continue;
		}

		let app_name = tokio::task::spawn_blocking(|| get_active_window().map(|w| w.app_name).unwrap_or_default()).await.unwrap_or_default();
		if previous.as_ref() != Some(&app_name) {
			previous = Some(app_name.clone());
			if tx.send(app_name).await.is_err() {
				return;
			}
		}

		tokio::time::sleep(if has_profiles { ACTIVE_INTERVAL } else { UI_ONLY_INTERVAL }).await;
	}
}
