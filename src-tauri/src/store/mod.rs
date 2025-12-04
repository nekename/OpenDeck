pub mod profiles;
mod simplified_profile;

use crate::shared::is_flatpak;

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use fs2::FileExt;
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
		let mut temp_file = fs::OpenOptions::new().write(true).create(true).truncate(true).open(&temp_path)?;
		FileExt::lock_exclusive(&temp_file)?;
		temp_file.write_all(contents.as_bytes())?;
		temp_file.sync_all()?;
		FileExt::unlock(&temp_file)?;
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

#[cfg(test)]
mod tests {
	use super::*;
	use tempfile::TempDir;

	#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
	struct TestData {
		pub value: String,
		pub number: i32,
	}

	impl NotProfile for TestData {}

	fn setup_test_dir() -> TempDir {
		TempDir::new().expect("Failed to create temp dir")
	}

	#[test]
	fn test_store_create_new_with_default() {
		let temp_dir = setup_test_dir();
		let default = TestData {
			value: "default".to_string(),
			number: 42,
		};

		let store = Store::new("test", temp_dir.path(), default.clone()).expect("Failed to create store");

		assert_eq!(store.value.value, "default");
		assert_eq!(store.value.number, 42);
	}

	#[test]
	fn test_store_save_and_load() {
		let temp_dir = setup_test_dir();
		let default = TestData {
			value: "test".to_string(),
			number: 100,
		};

		let store = Store::new("test", temp_dir.path(), default.clone()).expect("Failed to create store");
		store.save().expect("Failed to save store");

		// Verify file exists
		let json_path = temp_dir.path().join("test.json");
		assert!(json_path.exists(), "JSON file should exist after save");

		// Verify content is valid JSON
		let content = fs::read_to_string(&json_path).expect("Failed to read JSON file");
		let parsed: TestData = serde_json::from_str(&content).expect("JSON should be valid");
		assert_eq!(parsed.value, "test");
		assert_eq!(parsed.number, 100);

		// Load again and verify
		let loaded = Store::new("test", temp_dir.path(), TestData { value: "wrong".to_string(), number: 0 })
			.expect("Failed to load store");
		assert_eq!(loaded.value.value, "test");
		assert_eq!(loaded.value.number, 100);
	}

	#[test]
	fn test_store_recovery_from_temp_file() {
		let temp_dir = setup_test_dir();
		fs::create_dir_all(temp_dir.path()).ok();

		// Create corrupt main file
		let main_file = temp_dir.path().join("test.json");
		fs::write(&main_file, "{ corrupt json").expect("Failed to write corrupt file");

		// Create valid temp file
		let temp_file = temp_dir.path().join("test.json.temp");
		let valid_data = TestData {
			value: "from_temp".to_string(),
			number: 100,
		};
		fs::write(&temp_file, serde_json::to_string_pretty(&valid_data).unwrap()).expect("Failed to write temp file");

		// Load store - should recover from temp file
		let default = TestData {
			value: "default".to_string(),
			number: 0,
		};
		let store = Store::new("test", temp_dir.path(), default).expect("Failed to create store");

		assert_eq!(store.value.value, "from_temp");
		assert_eq!(store.value.number, 100);

		// Verify temp file was promoted to main
		assert!(main_file.exists(), "Main file should exist after recovery");
		assert!(!temp_file.exists(), "Temp file should be removed after recovery");
	}

	#[test]
	fn test_store_recovery_from_backup_file() {
		let temp_dir = setup_test_dir();
		fs::create_dir_all(temp_dir.path()).ok();

		// Create corrupt main and temp files
		fs::write(temp_dir.path().join("test.json"), "corrupt1").ok();
		fs::write(temp_dir.path().join("test.json.temp"), "corrupt2").ok();

		// Create valid backup file
		let backup_file = temp_dir.path().join("test.json.bak");
		let valid_data = TestData {
			value: "from_backup".to_string(),
			number: 200,
		};
		fs::write(&backup_file, serde_json::to_string_pretty(&valid_data).unwrap()).expect("Failed to write backup file");

		// Load store - should recover from backup
		let default = TestData {
			value: "default".to_string(),
			number: 0,
		};
		let store = Store::new("test", temp_dir.path(), default).expect("Failed to create store");

		assert_eq!(store.value.value, "from_backup");
		assert_eq!(store.value.number, 200);
	}

	#[test]
	fn test_store_uses_default_when_all_corrupt() {
		let temp_dir = setup_test_dir();
		fs::create_dir_all(temp_dir.path()).ok();

		// All files corrupt
		fs::write(temp_dir.path().join("test.json"), "corrupt1").ok();
		fs::write(temp_dir.path().join("test.json.temp"), "corrupt2").ok();
		fs::write(temp_dir.path().join("test.json.bak"), "corrupt3").ok();

		let default = TestData {
			value: "default".to_string(),
			number: 999,
		};
		let store = Store::new("test", temp_dir.path(), default).expect("Failed to create store");

		assert_eq!(store.value.value, "default");
		assert_eq!(store.value.number, 999);
	}

	#[test]
	fn test_store_cleanup_after_successful_load() {
		let temp_dir = setup_test_dir();
		fs::create_dir_all(temp_dir.path()).ok();

		// Create valid main file and extra temp/backup files
		let main_file = temp_dir.path().join("test.json");
		let temp_file = temp_dir.path().join("test.json.temp");
		let backup_file = temp_dir.path().join("test.json.bak");

		let data = TestData {
			value: "main".to_string(),
			number: 42,
		};
		fs::write(&main_file, serde_json::to_string_pretty(&data).unwrap()).ok();
		fs::write(&temp_file, "temp data").ok();
		fs::write(&backup_file, "backup data").ok();

		// Load store
		let _ = Store::new("test", temp_dir.path(), data).expect("Failed to create store");

		// Temp and backup should be cleaned up
		assert!(main_file.exists());
		assert!(!temp_file.exists(), "Temp file should be cleaned up");
		assert!(!backup_file.exists(), "Backup file should be cleaned up");
	}

	#[test]
	fn test_store_save_creates_backup() {
		let temp_dir = setup_test_dir();
		fs::create_dir_all(temp_dir.path()).ok();

		let main_file = temp_dir.path().join("test.json");
		let backup_file = temp_dir.path().join("test.json.bak");

		// Create initial store and save
		let data1 = TestData {
			value: "first".to_string(),
			number: 1,
		};
		let mut store = Store::new("test", temp_dir.path(), data1).expect("Failed to create store");
		store.save().expect("Failed to save first time");

		let first_content = fs::read_to_string(&main_file).unwrap();

		// Modify and save again
		store.value.value = "second".to_string();
		store.value.number = 2;
		store.save().expect("Failed to save second time");

		// Main file should have new content
		let second_content = fs::read_to_string(&main_file).unwrap();
		assert_ne!(first_content, second_content);

		// Backup should be cleaned up after successful save
		assert!(!backup_file.exists(), "Backup should be removed after successful save");
	}

	#[test]
	fn test_store_creates_parent_directories() {
		let temp_dir = setup_test_dir();
		let nested_path = temp_dir.path().join("nested").join("deeper").join("path");

		let data = TestData {
			value: "nested".to_string(),
			number: 42,
		};
		let store = Store::new("test", &nested_path, data).expect("Failed to create store");
		store.save().expect("Failed to save to nested path");

		assert!(nested_path.exists(), "Parent directories should be created");
		assert!(nested_path.join("test.json").exists(), "File should exist in nested path");
	}
}
