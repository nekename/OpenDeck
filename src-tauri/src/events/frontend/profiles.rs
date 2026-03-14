use super::Error;

use crate::shared::DEVICES;
use crate::store::profiles::{PROFILE_STORES, acquire_locks_mut, get_device_profiles};

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

	let selected_profile = locks.device_stores.get_selected_profile(&device)?;

	if selected_profile != id {
		let old_profile = &locks.profile_stores.get_profile_store(&DEVICES.get(&device).unwrap(), &selected_profile)?.value;
		for instance in old_profile.keys.iter().flatten().chain(&mut old_profile.sliders.iter().flatten()).chain(&mut old_profile.infobar.iter().flatten()) {
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
	for instance in new_profile.keys.iter().flatten().chain(&mut new_profile.sliders.iter().flatten()).chain(&mut new_profile.infobar.iter().flatten()) {
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
pub async fn rename_profile(device: String, old_id: String, new_id: String) -> Result<(), Error> {
	let mut locks = acquire_locks_mut().await;
	if !DEVICES.contains_key(&device) {
		return Err(Error::new(format!("device {device} not found")));
	}

	locks.profile_stores.rename_profile(&DEVICES.get(&device).unwrap(), &old_id, &new_id).await?;

	Ok(())
}

pub async fn rerender_images(app: &AppHandle) -> Result<(), anyhow::Error> {
	let window = app.get_webview_window("main").unwrap();
	window.emit("rerender_images", ())?;
	Ok(())
}
