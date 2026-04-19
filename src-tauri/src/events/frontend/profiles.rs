use super::Error;

use crate::shared::DEVICES;
use crate::store::profiles::{PROFILE_SAVE_DEBOUNCE, PROFILE_STORES, acquire_locks_mut, get_device_profiles, save_profile};

use tauri::{AppHandle, Emitter, Manager, command};

#[command]
pub fn get_profiles(device: &str) -> Result<Vec<String>, Error> {
	Ok(get_device_profiles(device)?)
}

#[command]
pub async fn get_selected_profile(device: String) -> Result<crate::shared::Profile, Error> {
	let mut locks = acquire_locks_mut().await;
	if !DEVICES.contains_key(&device) {
		return Err(Error::new(format!("device {device} not found")));
	}

	let selected_profile = locks.device_stores.get_selected_profile(&device)?;
	let profile = locks.profile_stores.get_profile_store(&DEVICES.get(&device).unwrap(), &selected_profile)?;

	Ok(profile.value.clone())
}

#[allow(clippy::flat_map_identity)]
#[command]
pub async fn set_selected_profile(device: String, id: String) -> Result<(), Error> {
	let mut locks = acquire_locks_mut().await;
	if !DEVICES.contains_key(&device) {
		return Err(Error::new(format!("device {device} not found")));
	}

	// If a profile save is pending for this device, save it immediately to prevent losing profile data
	let entries = PROFILE_SAVE_DEBOUNCE
		.iter()
		.filter(|entry| entry.key().device == device)
		.map(|entry| entry.key().clone())
		.collect::<Vec<_>>();
	if !entries.is_empty() {
		for context in &entries {
			if let Some((_, handle)) = PROFILE_SAVE_DEBOUNCE.remove(context) {
				handle.abort();
			}
		}
		if let Err(error) = save_profile(&device, &mut locks).await {
			log::error!("Failed to save profile for device {device}: {error}");
		}
	}

	let selected_profile = locks.device_stores.get_selected_profile(&device)?;

	if selected_profile != id {
		let old_profile = &locks.profile_stores.get_profile_store(&DEVICES.get(&device).unwrap(), &selected_profile)?.value;
		for instance in old_profile.keys.iter().flatten().chain(&mut old_profile.sliders.iter().flatten()) {
			if !matches!(instance.action.uuid.as_str(), "opendeck.multiaction" | "opendeck.toggleaction") {
				let _ = crate::events::outbound::will_appear::will_disappear(instance, false).await;
			} else {
				for child in instance.children.as_ref().unwrap() {
					let _ = crate::events::outbound::will_appear::will_disappear(child, false).await;
				}
			}
		}
		let _ = crate::events::outbound::devices::clear_screen(device.clone()).await;
	}

	// We must use the mutable version of get_profile_store in order to create the store if it does not exist.
	let store = locks.profile_stores.get_profile_store_mut(&DEVICES.get(&device).unwrap(), &id).await?;
	let new_profile = &store.value;

	// Lazy plugin activation: collect the plugin UUIDs referenced by the new
	// profile and ensure each has a running subprocess before firing
	// will_appear events. Plugins registered metadata-only at startup
	// (unreferenced) are spawned here on first use.
	let mut needed_plugins = std::collections::HashSet::<String>::new();
	for instance in new_profile.keys.iter().flatten().chain(&mut new_profile.sliders.iter().flatten()) {
		needed_plugins.insert(instance.action.plugin.clone());
		if let Some(children) = &instance.children {
			for child in children {
				needed_plugins.insert(child.action.plugin.clone());
			}
		}
	}
	for uuid in &needed_plugins {
		crate::plugins::ensure_plugin_spawned(uuid).await;
	}

	for instance in new_profile.keys.iter().flatten().chain(&mut new_profile.sliders.iter().flatten()) {
		if !matches!(instance.action.uuid.as_str(), "opendeck.multiaction" | "opendeck.toggleaction") {
			let _ = crate::events::outbound::will_appear::will_appear(instance).await;
		} else {
			for child in instance.children.as_ref().unwrap() {
				let _ = crate::events::outbound::will_appear::will_appear(child).await;
			}
		}
	}
	store.save()?;

	locks.device_stores.set_selected_profile(&device, id)?;

	// After a settle period, deactivate any plugin no longer needed by the
	// current profile on this device or any other connected device. Rapid
	// profile switching cancels and restarts the timer, so flipping through
	// profiles does not thrash plugin processes.
	drop(locks);
	crate::plugins::schedule_deactivation_sweep();

	Ok(())
}

#[command]
pub async fn delete_profile(device: String, profile: String) {
	let mut profile_stores = PROFILE_STORES.write().await;
	profile_stores.delete_profile(&device, &profile);
}

#[command]
pub async fn rename_profile(device: String, old_id: String, new_id: String, retain: bool) -> Result<(), Error> {
	let mut locks = acquire_locks_mut().await;
	if !DEVICES.contains_key(&device) {
		return Err(Error::new(format!("device {device} not found")));
	}

	locks.profile_stores.rename_profile(&DEVICES.get(&device).unwrap(), &old_id, &new_id, retain).await?;

	Ok(())
}

pub async fn rerender_images(app: &AppHandle) -> Result<(), anyhow::Error> {
	let window = app.get_webview_window("main").unwrap();
	window.emit("rerender_images", ())?;
	Ok(())
}
