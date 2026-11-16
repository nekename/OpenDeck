// Second window and a clipboard that both windows share.
//
// OpenDeck keeps its copied item in a Svelte store, which lives inside a single
// webview. A second window would therefore have its own, empty clipboard. The
// commands here keep the copied item in the Rust side and let every window read
// it, so copying in one window and pasting in the other works.

use std::sync::Mutex;

use tauri::{AppHandle, Emitter, Manager, WebviewUrl, WebviewWindowBuilder, command};

static CLIPBOARD: Mutex<Option<String>> = Mutex::new(None);

pub const SECONDARY_LABEL: &str = "secondary";

/// Opens the second window, or brings it to the front if it is already open.
#[command]
pub async fn open_second_window(app: AppHandle) -> Result<(), String> {
	let existing = app.get_webview_window(SECONDARY_LABEL);
	if let Some(window) = existing {
		let _ = window.set_focus();
		return Ok(());
	}

	WebviewWindowBuilder::new(
		&app,
		SECONDARY_LABEL,
		WebviewUrl::App("index.html".into()),
	)
	.title("OpenDeck")
	.inner_size(800.0, 600.0)
	.min_inner_size(800.0, 600.0)
	.center()
	.build()
	.map(|_| {
		let _ = app.emit("second_window", true);
	})
	.map_err(|error| error.to_string())
}

/// Emits the current state after a window change, so the button can follow.
pub fn emit_state(app: &AppHandle) {
	let open = app.get_webview_window(SECONDARY_LABEL).is_some();
	let _ = app.emit("second_window", open);
}

/// Opens or closes the second window and returns the new state.
#[command]
pub async fn toggle_second_window(app: AppHandle) -> Result<bool, String> {
	if app.get_webview_window(SECONDARY_LABEL).is_some() {
		close_second_window(app.clone()).await;
		Ok(false)
	} else {
		open_second_window(app).await?;
		Ok(true)
	}
}

/// Closes the second window, used when the setting is turned off.
#[command]
pub async fn close_second_window(app: AppHandle) {
	if let Some(window) = app.get_webview_window(SECONDARY_LABEL) {
		let _ = window.close();
	}
	let _ = app.emit("second_window", false);
}

/// Stores the copied item and tells every window about it.
#[command]
pub async fn set_clipboard(app: AppHandle, item: Option<String>) {
	*CLIPBOARD.lock().unwrap() = item.clone();
	let _ = app.emit("clipboard", item);
}

/// Returns the copied item, so a window that was not focused can paste it.
#[command]
pub async fn get_clipboard() -> Option<String> {
	CLIPBOARD.lock().unwrap().clone()
}
