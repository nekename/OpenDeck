use std::sync::atomic::{AtomicBool, Ordering};

use tauri::{AppHandle, image::Image};

/// Whether the taskbar or panel is dark, which decides the automatic monochrome tray icon.
static DARK_PANEL: AtomicBool = AtomicBool::new(true);

/// Returns the tray icon for the given style and whether it should be treated as a template image on macOS.
pub fn icon(app: &AppHandle, style: &str) -> (Image<'static>, bool) {
	let style = match style {
		"auto" if DARK_PANEL.load(Ordering::Relaxed) => "light",
		"auto" => "dark",
		style => style,
	};

	match style {
		"light" => (tauri::include_image!("icons/tray-light.png"), true),
		"dark" => (tauri::include_image!("icons/tray-dark.png"), true),
		_ => (app.default_window_icon().unwrap().clone().to_owned(), false),
	}
}

pub fn set_icon(app: &AppHandle, style: &str) -> Result<(), tauri::Error> {
	let Some(tray) = app.tray_by_id("opendeck") else { return Ok(()) };
	let (icon, template) = icon(app, style);
	tray.set_icon(Some(icon))?;
	tray.set_icon_as_template(template)
}

#[cfg(target_os = "linux")]
async fn panel_is_dark(connection: &zbus::Connection) -> Option<bool> {
	// GNOME Shell's top bar is dark regardless of the colour scheme
	if std::env::var("XDG_CURRENT_DESKTOP").is_ok_and(|desktop| desktop.split(':').any(|desktop| desktop == "GNOME")) {
		return Some(true);
	}

	// Elsewhere, assume that the panel follows the colour scheme reported by the XDG desktop portal
	let reply = connection
		.call_method(
			Some("org.freedesktop.portal.Desktop"),
			"/org/freedesktop/portal/desktop",
			Some("org.freedesktop.portal.Settings"),
			"ReadOne",
			&("org.freedesktop.appearance", "color-scheme"),
		)
		.await
		.ok()?;
	let value: zbus::zvariant::OwnedValue = reply.body().deserialize().ok()?;
	// 0 is no preference, 1 is prefer dark and 2 is prefer light
	Some(u32::try_from(value).ok()? == 1)
}

#[cfg(windows)]
fn panel_is_dark() -> Option<bool> {
	use winreg::{RegKey, enums::HKEY_CURRENT_USER};
	let personalize = RegKey::predef(HKEY_CURRENT_USER).open_subkey(r"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize");
	// The value is absent on versions of Windows without a light taskbar
	Some(personalize.and_then(|key| key.get_value::<u32, _>("SystemUsesLightTheme")).map_or(true, |value| value == 0))
}

/// Polls the taskbar or panel theme, updating the tray icon if the automatic style is selected.
/// On macOS, monochrome icons are template images that the system recolours, so no detection is needed.
pub fn init_panel_watcher() {
	#[cfg(any(target_os = "linux", windows))]
	tokio::spawn(async {
		#[cfg(target_os = "linux")]
		let connection = match zbus::Connection::session().await {
			Ok(connection) => connection,
			Err(error) => {
				log::warn!("Failed to connect to the session bus to detect the panel theme: {error}");
				return;
			}
		};

		loop {
			#[cfg(target_os = "linux")]
			let dark = panel_is_dark(&connection).await;
			#[cfg(windows)]
			let dark = panel_is_dark();

			if let Some(dark) = dark
				&& DARK_PANEL.swap(dark, Ordering::Relaxed) != dark
				&& crate::store::SETTINGS_MUT.lock().await.value.trayicon == "auto"
				&& let Err(error) = set_icon(crate::APP_HANDLE.get().unwrap(), "auto")
			{
				log::error!("Failed to update tray icon: {error}");
			}

			tokio::time::sleep(std::time::Duration::from_secs(2)).await;
		}
	});
}
