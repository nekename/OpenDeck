use super::Error;

use crate::built_info;
use crate::shared::{PRODUCT_NAME, config_dir};

use std::fs::{File, read_dir};
use std::io::Write;
use std::path::{Path, PathBuf};

use path_slash::PathExt;
use tauri::{AppHandle, Manager, command};
#[cfg(not(debug_assertions))]
use tauri_plugin_autostart::ManagerExt;
use tauri_plugin_dialog::{DialogExt, FilePath};
use zip::{ZipWriter, write::FileOptions};

#[command]
pub async fn get_settings() -> Result<crate::store::Settings, Error> {
	let store = crate::store::get_settings();
	match store {
		Ok(store) => Ok(store.value),
		Err(error) => Err(error.into()),
	}
}

#[command]
pub async fn set_settings(_app: AppHandle, settings: crate::store::Settings) -> Result<(), Error> {
	#[cfg(not(debug_assertions))]
	let _ = match settings.autolaunch {
		true => _app.autolaunch().enable(),
		false => _app.autolaunch().disable(),
	};

	crate::events::outbound::devices::set_brightness(settings.brightness).await?;
	crate::device_sleep::update_timeout_minutes(settings.sleep_timeout_minutes);
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
	if let Err(error) = open::that_detached(config_dir()) {
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
		PRODUCT_NAME,
		built_info::PKG_VERSION,
		built_info::GIT_COMMIT_HASH_SHORT.unwrap_or("commit hash unknown"),
		built_info::TARGET,
		built_info::DIRECT_DEPENDENCIES_STR
	)
}

fn add_dir_to_zip<W: Write + std::io::Seek>(zip: &mut ZipWriter<W>, base_dir: &Path, current_dir: &Path, options: FileOptions<()>, skip_paths: &[PathBuf]) -> std::io::Result<()> {
	for entry in read_dir(current_dir)? {
		let entry = entry?;
		let path = entry.path();
		if skip_paths.contains(&path) {
			continue;
		}
		let relative = path.strip_prefix(base_dir).unwrap();

		if path.is_dir() {
			zip.add_directory(relative.to_slash_lossy(), options)?;
			add_dir_to_zip(zip, base_dir, &path, options, skip_paths)?;
		} else {
			zip.start_file(relative.to_slash_lossy(), options)?;
			let mut file = File::open(&path)?;
			std::io::copy(&mut file, zip)?;
		}
	}
	Ok(())
}

#[command]
pub async fn backup_config_directory(app: AppHandle) -> Result<bool, Error> {
	let filename = format!(
		"{}_config_{}_{}_{}.zip",
		PRODUCT_NAME,
		std::env::consts::OS,
		std::env::consts::ARCH.replace("_", "-"),
		chrono::Local::now().format("%Y%m%d")
	);

	let path = app
		.dialog()
		.file()
		.set_file_name(filename)
		.add_filter(format!("{} config backup", PRODUCT_NAME), &["zip"])
		.blocking_save_file();

	let Some(FilePath::Path(path)) = path else {
		return Ok(false);
	};
	let _ = std::fs::remove_file(&path);

	let temp_path = path.with_extension("zip.part");
	let file = File::create(&temp_path)?;

	let mut skip_paths = vec![temp_path.clone()];
	let config_dir = config_dir();

	if let Some(builtin_plugins) = app
		.path()
		.resolve("plugins", tauri::path::BaseDirectory::Resource)
		.ok()
		.and_then(|p| read_dir(p).ok())
		.map(|e| e.into_iter().flatten().map(|x| config_dir.join("plugins").join(x.file_name())).collect::<Vec<_>>())
	{
		skip_paths.extend(builtin_plugins);
	};

	let mut zip = ZipWriter::new(file);
	let options = FileOptions::default().compression_method(zip::CompressionMethod::Deflated);
	add_dir_to_zip(&mut zip, &config_dir, &config_dir, options, &skip_paths)?;
	zip.finish().map_err(anyhow::Error::from)?;

	std::fs::rename(temp_path, path)?;

	Ok(true)
}

#[command]
pub async fn restore_config_directory(app: AppHandle) -> Result<(), Error> {
	let path = app.dialog().file().add_filter(format!("{} config backup", PRODUCT_NAME), &["zip"]).blocking_pick_file();

	let Some(FilePath::Path(path)) = path else {
		return Ok(());
	};

	let config_dir = config_dir();
	let temp_dir = config_dir.with_extension("temp");
	let backup_dir = config_dir.with_extension("bak");
	let _ = std::fs::remove_dir_all(&temp_dir);
	let _ = std::fs::remove_dir_all(&backup_dir);

	crate::zip_extract::extract(File::open(path)?, &temp_dir).map_err(anyhow::Error::from)?;
	#[cfg(windows)]
	crate::plugins::deactivate_plugins().await;
	std::fs::rename(&config_dir, &backup_dir)?;
	std::fs::rename(temp_dir, &config_dir)?;
	let _ = std::fs::remove_dir_all(backup_dir);

	app.restart();
}
