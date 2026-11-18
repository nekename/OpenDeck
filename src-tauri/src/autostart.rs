//! Drop-in replacement for the subset of `tauri-plugin-autostart` used by OpenDeck, which (via `auto-launch`) has no
//! FreeBSD backend. Writes an XDG autostart entry like `auto-launch` does on Linux, except that it honours
//! `$XDG_CONFIG_HOME`, quotes the executable path, and uses the product name for `Name=`.

#![cfg_attr(debug_assertions, expect(dead_code, reason = "only used by `set_settings` in release builds, as with tauri-plugin-autostart"))]

use std::path::PathBuf;

use tauri::plugin::TauriPlugin;
use tauri::{Manager, Runtime, State};

#[derive(Debug, Default, Clone, Copy)]
#[expect(dead_code, reason = "mirrors tauri-plugin-autostart's API; only `LaunchAgent` is used")]
pub enum MacosLauncher {
	#[default]
	LaunchAgent,
	AppleScript,
}

pub struct AutoLaunchManager {
	app_name: String,
	args: Vec<String>,
}

impl AutoLaunchManager {
	fn entry_path(&self) -> Result<PathBuf, String> {
		std::env::var("XDG_CONFIG_HOME")
			.map(PathBuf::from)
			.ok()
			.or_else(|| std::env::var("HOME").ok().map(|h| PathBuf::from(h).join(".config")))
			.map(|config| config.join("autostart").join(format!("{}.desktop", self.app_name)))
			.ok_or_else(|| "unable to determine the XDG config directory".to_owned())
	}

	pub fn enable(&self) -> Result<(), String> {
		let path = self.entry_path()?;
		let exe = tauri::utils::platform::current_exe().map_err(|e| e.to_string())?;
		let exec = format!("\"{}\" {}", exe.display(), self.args.join(" "));
		let dir = path.parent().ok_or("autostart entry path has no parent directory")?;
		std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
		std::fs::write(
			&path,
			format!(
				"[Desktop Entry]\nType=Application\nVersion=1.0\nName={name}\nComment={name} startup script\nExec={exec}\nStartupNotify=false\nTerminal=false\n",
				name = crate::shared::PRODUCT_NAME
			),
		)
		.map_err(|e| e.to_string())
	}

	pub fn disable(&self) -> Result<(), String> {
		let path = self.entry_path()?;
		if path.exists() {
			std::fs::remove_file(path).map_err(|e| e.to_string())?;
		}
		Ok(())
	}
}

pub trait ManagerExt<R: Runtime> {
	fn autolaunch(&self) -> State<'_, AutoLaunchManager>;
}

impl<R: Runtime, T: Manager<R>> ManagerExt<R> for T {
	fn autolaunch(&self) -> State<'_, AutoLaunchManager> {
		self.state::<AutoLaunchManager>()
	}
}

pub fn init<R: Runtime>(_macos_launcher: MacosLauncher, args: Option<Vec<&'static str>>) -> TauriPlugin<R> {
	let args = args.unwrap_or_default().into_iter().map(str::to_owned).collect();
	tauri::plugin::Builder::new("autostart")
		.setup(move |app, _api| {
			app.manage(AutoLaunchManager {
				app_name: app.package_info().name.clone(),
				args,
			});
			Ok(())
		})
		.build()
}
