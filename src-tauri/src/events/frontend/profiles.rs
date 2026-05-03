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

/// Set one swipe neighbor with bidirectional enforcement.
/// direction = "left" or "right". target = profile to link to.
/// When profile X sets swipe_right = Y, profile Y's swipe_left is
/// automatically set to X. The old neighbor's reciprocal is cleared.
#[command]
pub async fn set_swipe_neighbor(device: String, profile: String, direction: String, target: String) -> Result<(), Error> {
	let mut locks = acquire_locks_mut().await;
	if !DEVICES.contains_key(&device) {
		return Err(Error::new(format!("device {device} not found")));
	}
	let device_info = DEVICES.get(&device).unwrap();
	let is_left = direction == "left";

	let store = locks.profile_stores.get_profile_store_mut(&device_info, &profile).await?;
	let old_target = if is_left { store.value.swipe_left.clone() } else { store.value.swipe_right.clone() };

	if is_left { store.value.swipe_left = Some(target.clone()); }
	else { store.value.swipe_right = Some(target.clone()); }
	store.save()?;

	// Clear old neighbor's reciprocal (if it pointed back to us)
	if let Some(ref old) = old_target {
		if *old != target {
			if let Ok(neighbor) = locks.profile_stores.get_profile_store_mut(&device_info, old).await {
				let recip = if is_left { &mut neighbor.value.swipe_right } else { &mut neighbor.value.swipe_left };
				if recip.as_ref() == Some(&profile) {
					*recip = None;
					let _ = neighbor.save();
				}
			}
		}
	}

	// Set new neighbor's reciprocal
	if let Ok(neighbor) = locks.profile_stores.get_profile_store_mut(&device_info, &target).await {
		let recip = if is_left { &mut neighbor.value.swipe_right } else { &mut neighbor.value.swipe_left };
		*recip = Some(profile.clone());
		let _ = neighbor.save();
	}

	Ok(())
}

pub async fn rerender_images(app: &AppHandle) -> Result<(), anyhow::Error> {
	let window = app.get_webview_window("main").unwrap();
	window.emit("rerender_images", ())?;
	Ok(())
}
