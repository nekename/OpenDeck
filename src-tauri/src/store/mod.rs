pub mod profiles;
mod simplified_profile;

use crate::shared::is_flatpak;

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

pub trait FromAndIntoDiskValue
where
	Self: Sized,
{
	#[allow(clippy::wrong_self_convention)]
	fn into_value(&self) -> Result<serde_json::Value, serde_json::Error>;
	fn from_value(_: serde_json::Value, _: &Path) -> Result<Self, serde_json::Error>;
}

pub trait NotProfile {}

impl<T> FromAndIntoDiskValue for T
where
	T: Serialize + for<'a> Deserialize<'a> + NotProfile,
{
	fn into_value(&self) -> Result<serde_json::Value, serde_json::Error> {
		serde_json::to_value(self)
	}
	fn from_value(value: serde_json::Value, _: &Path) -> Result<T, serde_json::Error> {
		serde_json::from_value(value)
	}
}

/// Allows for easy persistence of values using JSON files
pub struct Store<T>
where
	T: FromAndIntoDiskValue,
{
	pub value: T,
	path: PathBuf,
}

impl<T> Store<T>
where
	T: FromAndIntoDiskValue,
{
	/// Validate that a file contains valid data for type T
	fn validate_file_contents(path: &Path) -> Result<T, anyhow::Error> {
		let file_contents = fs::read(path)?;
		let value: T = T::from_value(serde_json::from_slice(&file_contents)?, path)?;
		Ok(value)
	}

	/// Create a new Store given an ID and storage directory
	pub fn new(id: &str, config_dir: &Path, default: T) -> Result<Self, anyhow::Error> {
		let path = config_dir.join(format!("{}.json", id));
		let temp_path = path.with_extension("json.temp");
		let backup_path = path.with_extension("json.bak");

		if let Ok(value) = Self::validate_file_contents(&path) {
			let _ = fs::remove_file(&temp_path);
			let _ = fs::remove_file(&backup_path);
			Ok(Self { path, value })
		} else if let Ok(value) = Self::validate_file_contents(&temp_path) {
			fs::rename(&temp_path, &path)?;
			Ok(Self { path, value })
		} else if let Ok(value) = Self::validate_file_contents(&backup_path) {
			fs::rename(&backup_path, &path)?;
			Ok(Self { path, value })
		} else {
			Ok(Self { path, value: default })
		}
	}

	/// Save the relevant Store as a file
	pub fn save(&self) -> Result<(), anyhow::Error> {
		fs::create_dir_all(self.path.parent().unwrap())?;

		let contents = serde_json::to_string_pretty(&T::into_value(&self.value)?)?;

		let temp_path = self.path.with_extension("json.temp");
		let backup_path = self.path.with_extension("json.bak");

		// Write to temporary file
		let mut temp_file = fs::OpenOptions::new().write(true).truncate(true).create(true).open(&temp_path)?;
		temp_file.lock()?;
		temp_file.write_all(contents.as_bytes())?;
		temp_file.sync_data()?;
		temp_file.unlock()?;
		drop(temp_file);

		// If main file exists, back it up
		if self.path.exists() {
			fs::rename(&self.path, &backup_path)?;
		}

		// Rename temp file to main file
		fs::rename(&temp_path, &self.path)?;

		// Remove backup file if everything succeeded
		if backup_path.exists() {
			let _ = fs::remove_file(&backup_path);
		}

		Ok(())
	}
}

#[derive(Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
	pub version: String,
	pub language: String,
	pub brightness: u8,
	pub rotation: u16,
	pub darktheme: bool,
	pub background: bool,
	pub autolaunch: bool,
	pub updatecheck: bool,
	pub statistics: bool,
	pub separatewine: bool,
	pub developer: bool,
	pub disableelgato: bool,
}

impl Default for Settings {
	fn default() -> Self {
		Self {
			version: "0.0.0".to_owned(),
			language: "en".to_owned(),
			brightness: 50,
			rotation: 0,
			darktheme: true,
			background: !is_flatpak(),
			autolaunch: false,
			updatecheck: option_env!("OPENDECK_DISABLE_UPDATE_CHECK").is_none() && !is_flatpak(),
			// Consent is given by the user on install so it is OK to have the default be `true`
			statistics: true,
			separatewine: false,
			developer: false,
			disableelgato: false,
		}
	}
}

impl NotProfile for Settings {}

pub fn get_settings() -> Result<Store<Settings>, anyhow::Error> {
	Store::new("settings", &crate::shared::config_dir(), Settings::default())
}
