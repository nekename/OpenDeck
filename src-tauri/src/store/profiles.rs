use super::{
	Store,
	simplified_profile::{DiskActionInstance, DiskProfile},
};
use crate::shared::{Action, ActionInstance, ActionState, DEVICES, DeviceInfo, Profile, config_dir};

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use anyhow::{Context, anyhow};
use once_cell::sync::Lazy;
use tokio::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};

pub struct ProfileStores {
	stores: HashMap<String, Store<Profile>>,
}

impl ProfileStores {
	fn canonical_id(device: &str, id: &str) -> String {
		if cfg!(target_os = "windows") {
			PathBuf::from(device).join(id.replace('/', "\\")).to_str().unwrap().to_owned()
		} else {
			PathBuf::from(device).join(id).to_str().unwrap().to_owned()
		}
	}

	pub fn get_profile_store(&self, device: &DeviceInfo, id: &str) -> Result<&Store<Profile>, anyhow::Error> {
		self.stores.get(&Self::canonical_id(&device.id, id)).ok_or_else(|| anyhow!("profile not found"))
	}

	pub async fn get_profile_store_mut(&mut self, device: &DeviceInfo, id: &str) -> Result<&mut Store<Profile>, anyhow::Error> {
		let canonical_id = Self::canonical_id(&device.id, id);
		if self.stores.contains_key(&canonical_id) {
			Ok(self.stores.get_mut(&canonical_id).unwrap())
		} else {
			let default = Profile {
				id: id.to_owned(),
				keys: Vec::new(),
				sliders: Vec::new(),
			};

			let mut store = Store::new(&canonical_id, &config_dir().join("profiles"), default).context(format!("Failed to create store for profile {}", canonical_id))?;
			store.value.keys.resize((device.rows * device.columns) as usize, None);
			store.value.sliders.resize(device.encoders as usize, None);

			let categories = crate::shared::CATEGORIES.read().await;
			let actions = categories.values().flatten().collect::<Vec<_>>();
			let plugins_dir = config_dir().join("plugins");
			let registered = crate::events::registered_plugins().await;
			let keep_instance = |instance: &ActionInstance| -> bool {
				instance.action.plugin == "opendeck"
					|| (plugins_dir.join(&instance.action.plugin).exists() && (!registered.contains(&instance.action.plugin) || actions.iter().any(|v| v.uuid == instance.action.uuid)))
			};
			for slot in store.value.keys.iter_mut().chain(store.value.sliders.iter_mut()) {
				if let Some(instance) = slot {
					if !keep_instance(instance) {
						*slot = None;
					} else if let Some(children) = &mut instance.children {
						children.retain_mut(|child| keep_instance(child));
					}
				}
			}
			store.save()?;

			self.stores.insert(canonical_id.clone(), store);
			Ok(self.stores.get_mut(&canonical_id).unwrap())
		}
	}

	pub fn remove_profile(&mut self, device: &str, id: &str) {
		self.stores.remove(&Self::canonical_id(device, id));
	}

	pub fn delete_profile(&mut self, device: &str, id: &str) {
		self.remove_profile(device, id);
		let config_dir = config_dir();
		#[cfg(target_os = "windows")]
		let id = &id.replace('/', "\\");
		let path = config_dir.join("profiles").join(device).join(format!("{id}.json"));
		let _ = fs::remove_file(&path);
		// This is safe as `remove_dir` errors if the directory is not empty.
		let _ = fs::remove_dir(path.parent().unwrap());
		let images_path = config_dir.join("images").join(device).join(id);
		let _ = fs::remove_dir_all(images_path);
	}

	pub fn all_from_plugin(&self, plugin: &str) -> Vec<crate::shared::ActionContext> {
		let mut all = vec![];
		for store in self.stores.values() {
			for instance in store.value.keys.iter().chain(&store.value.sliders).flatten() {
				if instance.action.plugin == plugin {
					all.push(instance.context.clone());
				}
			}
		}
		all
	}
}

#[derive(Serialize, Deserialize)]
pub struct DeviceConfig {
	pub selected_profile: String,
}

impl super::NotProfile for DeviceConfig {}

pub struct DeviceStores {
	stores: HashMap<String, Store<DeviceConfig>>,
}

impl DeviceStores {
	pub fn get_selected_profile(&mut self, device: &str) -> Result<String, anyhow::Error> {
		if !self.stores.contains_key(device) {
			let default = DeviceConfig {
				selected_profile: "Default".to_owned(),
			};

			let store = Store::new(device, &config_dir().join("profiles"), default).context(format!("Failed to create store for device config {}", device))?;
			store.save()?;

			self.stores.insert(device.to_owned(), store);
		}

		let from_store = &self.stores.get(device).unwrap().value.selected_profile;
		let all = get_device_profiles(device)?;
		if all.contains(from_store) { Ok(from_store.clone()) } else { Ok(all.first().unwrap().clone()) }
	}

	pub fn set_selected_profile(&mut self, device: &str, id: String) -> Result<(), anyhow::Error> {
		if self.stores.contains_key(device) {
			let store = self.stores.get_mut(device).unwrap();
			store.value.selected_profile = id;
			store.save()?;
		} else {
			let default = DeviceConfig { selected_profile: id };

			let store = Store::new(device, &config_dir().join("profiles"), default).context(format!("Failed to create store for device config {}", device))?;
			store.save()?;

			self.stores.insert(device.to_owned(), store);
		}
		Ok(())
	}
}

#[derive(Deserialize)]
#[allow(dead_code)]
struct ProfileV1 {
	id: String,
	keys: Vec<Vec<ActionInstance>>,
	sliders: Vec<Vec<ActionInstance>>,
}

impl From<ProfileV1> for DiskProfile {
	fn from(val: ProfileV1) -> Self {
		let mut keys = vec![];
		for slot in val.keys {
			if slot.len() == 1 {
				keys.push(Some(slot[0].clone().into()));
			} else if !slot.is_empty() {
				let mut children = slot.clone();
				for child in &mut children {
					child.context.index += 1;
				}
				keys.push(Some(DiskActionInstance {
					action: Action {
						name: "Multi Action".to_owned(),
						uuid: "opendeck.multiaction".to_owned(),
						plugin: "opendeck".to_owned(),
						tooltip: "Execute multiple actions".to_owned(),
						icon: "opendeck/multi-action.png".to_owned(),
						disable_automatic_states: false,
						visible_in_action_list: true,
						supported_in_multi_actions: false,
						property_inspector: String::new(),
						controllers: vec!["Keypad".to_owned()],
						states: vec![ActionState {
							image: "opendeck/multi-action.png".to_owned(),
							..Default::default()
						}],
					},
					context: slot[0].context.clone().into(),
					states: vec![ActionState {
						image: "opendeck/multi-action.png".to_owned(),
						..Default::default()
					}],
					current_state: 0,
					settings: Value::Object(serde_json::Map::new()),
					children: Some(children.into_iter().map(|v| v.into()).collect()),
				}));
			} else {
				keys.push(None);
			}
		}
		let mut sliders = vec![];
		for slot in val.sliders {
			if !slot.is_empty() {
				sliders.push(Some(slot[0].clone().into()));
			} else {
				sliders.push(None);
			}
		}
		Self { keys, sliders }
	}
}

#[derive(Deserialize)]
#[serde(untagged)]
#[allow(dead_code)]
enum ProfileVersions {
	V1(ProfileV1),
	V2(Profile),
	V3(DiskProfile),
}

fn migrate_profile(path: PathBuf) -> Result<(), anyhow::Error> {
	let profile = serde_json::from_slice(&fs::read(&path)?)?;
	let migrated: DiskProfile = match profile {
		ProfileVersions::V1(v1) => v1.into(),
		ProfileVersions::V2(v2) => (&v2).into(),
		ProfileVersions::V3(v3) => v3,
	};
	let mut as_value = serde_json::to_value(migrated)?;
	fn replace_old_identifier(value: &mut Value) {
		match value {
			Value::String(v) => *v = v.replace("com.amansprojects.opendeck", "opendeck"),
			Value::Object(v) => {
				for value in v.values_mut() {
					replace_old_identifier(value);
				}
			}
			Value::Array(v) => {
				for value in v.iter_mut() {
					replace_old_identifier(value);
				}
			}
			_ => (),
		}
	}
	replace_old_identifier(&mut as_value);
	fs::write(path, serde_json::to_string_pretty(&as_value)?)?;
	Ok(())
}

pub fn get_device_profiles(device: &str) -> Result<Vec<String>, anyhow::Error> {
	let mut profiles: Vec<String> = vec![];

	let device_path = config_dir().join("profiles").join(device);
	fs::create_dir_all(&device_path)?;
	let entries = fs::read_dir(device_path)?;

	for entry in entries.flatten() {
		if entry.metadata()?.is_file() {
			let mut id = entry.file_name().to_string_lossy().into_owned();
			if !id.ends_with(".json") {
				continue;
			}
			id.truncate(id.len() - 5);
			if let Err(error) = migrate_profile(entry.path()) {
				log::warn!("Failed to migrate profile {id}: {error}");
			} else {
				profiles.push(id);
			}
		} else if entry.metadata()?.is_dir() {
			let entries = fs::read_dir(entry.path())?;
			for subentry in entries.flatten() {
				if subentry.metadata()?.is_file() {
					let mut id = format!("{}/{}", entry.file_name().to_string_lossy(), &subentry.file_name().to_string_lossy());
					if !id.ends_with(".json") {
						continue;
					}
					id.truncate(id.len() - 5);
					if let Err(error) = migrate_profile(subentry.path()) {
						log::warn!("Failed to migrate profile {id}: {error}");
					} else {
						profiles.push(id);
					}
				}
			}
		}
	}

	if profiles.is_empty() {
		profiles.push("Default".to_owned());
	}

	Ok(profiles)
}

/// A singleton object to contain all active Store instances that hold a profile.
pub static PROFILE_STORES: Lazy<RwLock<ProfileStores>> = Lazy::new(|| RwLock::new(ProfileStores { stores: HashMap::new() }));

/// A singleton object to manage Store instances for device configurations.
pub static DEVICE_STORES: Lazy<RwLock<DeviceStores>> = Lazy::new(|| RwLock::new(DeviceStores { stores: HashMap::new() }));

pub struct Locks<'a> {
	#[allow(dead_code)]
	pub device_stores: RwLockReadGuard<'a, DeviceStores>,
	pub profile_stores: RwLockReadGuard<'a, ProfileStores>,
}

pub async fn acquire_locks() -> Locks<'static> {
	let device_stores = DEVICE_STORES.read().await;
	let profile_stores = PROFILE_STORES.read().await;
	Locks { device_stores, profile_stores }
}

pub struct LocksMut<'a> {
	pub device_stores: RwLockWriteGuard<'a, DeviceStores>,
	pub profile_stores: RwLockWriteGuard<'a, ProfileStores>,
}

pub async fn acquire_locks_mut() -> LocksMut<'static> {
	let device_stores = DEVICE_STORES.write().await;
	let profile_stores = PROFILE_STORES.write().await;
	LocksMut { device_stores, profile_stores }
}

pub async fn get_slot<'a>(context: &crate::shared::Context, locks: &'a Locks<'_>) -> Result<&'a Option<crate::shared::ActionInstance>, anyhow::Error> {
	let device = DEVICES.get(&context.device).ok_or_else(|| anyhow!("device not found"))?;
	let store = locks.profile_stores.get_profile_store(&device, &context.profile)?;

	let configured = match &context.controller[..] {
		"Encoder" => store.value.sliders.get(context.position as usize).ok_or_else(|| anyhow!("index out of bounds"))?,
		_ => store.value.keys.get(context.position as usize).ok_or_else(|| anyhow!("index out of bounds"))?,
	};

	Ok(configured)
}

pub async fn get_slot_mut<'a>(context: &crate::shared::Context, locks: &'a mut LocksMut<'_>) -> Result<&'a mut Option<crate::shared::ActionInstance>, anyhow::Error> {
	let device = DEVICES.get(&context.device).ok_or_else(|| anyhow!("device not found"))?;
	let store = locks.profile_stores.get_profile_store_mut(&device, &context.profile).await?;

	let configured = match &context.controller[..] {
		"Encoder" => store.value.sliders.get_mut(context.position as usize).ok_or_else(|| anyhow!("index out of bounds"))?,
		_ => store.value.keys.get_mut(context.position as usize).ok_or_else(|| anyhow!("index out of bounds"))?,
	};

	Ok(configured)
}

pub async fn get_instance<'a>(context: &crate::shared::ActionContext, locks: &'a Locks<'_>) -> Result<Option<&'a crate::shared::ActionInstance>, anyhow::Error> {
	let slot = get_slot(&(context.into()), locks).await?;
	if let Some(instance) = slot {
		if instance.context == *context {
			return Ok(Some(instance));
		} else if let Some(children) = &instance.children {
			for child in children {
				if child.context == *context {
					return Ok(Some(child));
				}
			}
		}
	}
	Ok(None)
}

pub async fn get_instance_mut<'a>(context: &crate::shared::ActionContext, locks: &'a mut LocksMut<'_>) -> Result<Option<&'a mut crate::shared::ActionInstance>, anyhow::Error> {
	let slot = get_slot_mut(&(context.into()), locks).await?;
	if let Some(instance) = slot {
		if instance.context == *context {
			return Ok(Some(instance));
		} else if let Some(children) = &mut instance.children {
			for child in children {
				if child.context == *context {
					return Ok(Some(child));
				}
			}
		}
	}
	Ok(None)
}

pub async fn save_profile(device: &str, locks: &mut LocksMut<'_>) -> Result<(), anyhow::Error> {
	let selected_profile = locks.device_stores.get_selected_profile(device)?;
	let device = DEVICES.get(device).unwrap();
	let store = locks.profile_stores.get_profile_store(&device, &selected_profile)?;
	store.save()
}
