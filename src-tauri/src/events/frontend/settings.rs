use super::Error;

use crate::built_info;

use tauri::command;
#[cfg(not(debug_assertions))]
use tauri_plugin_autostart::ManagerExt;

#[command]
pub async fn get_settings() -> Result<crate::store::Settings, Error> {
	let store = crate::store::get_settings();
	match store {
		Ok(store) => Ok(store.value),
		Err(error) => Err(error.into()),
	}
}

#[command]
pub async fn set_settings(_app: tauri::AppHandle, settings: crate::store::Settings) -> Result<(), Error> {
	#[cfg(not(debug_assertions))]
	let _ = match settings.autolaunch {
		true => _app.autolaunch().enable(),
		false => _app.autolaunch().disable(),
	};

	crate::events::outbound::devices::set_brightness(settings.brightness).await?;
	let mut store = match crate::store::get_settings() {
		Ok(store) => store,
		Err(error) => return Err(error.into()),
	};

	store.value = settings;
	store.save()?;
	Ok(())
}

#[command]
pub fn open_config_directory() -> Result<(), Error> {
	if let Err(error) = open::that_detached(crate::shared::config_dir()) {
		return Err(anyhow::Error::from(error).into());
	}
	Ok(())
}

#[command]
pub fn open_log_directory() -> Result<(), Error> {
	if let Err(error) = open::that_detached(crate::shared::log_dir()) {
		return Err(anyhow::Error::from(error).into());
	}
	Ok(())
}

#[command]
pub fn get_build_info() -> String {
	format!(
		r#"
		<details>
			<summary> {} v{} ({}) on {} </summary>
			{}
		</details>
		"#,
		crate::shared::PRODUCT_NAME,
		built_info::PKG_VERSION,
		built_info::GIT_COMMIT_HASH_SHORT.unwrap_or("commit hash unknown"),
		built_info::TARGET,
		built_info::DIRECT_DEPENDENCIES_STR
	)
}
