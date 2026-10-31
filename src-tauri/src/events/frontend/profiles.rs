use super::Error;

use crate::shared::{DEVICES, Profile, ProfilePage};
use crate::store::profiles::{PROFILE_STORES, acquire_locks_mut, get_device_profiles, save_profile_now};

use tauri::{AppHandle, Emitter, Manager, command};

#[command]
pub fn get_profiles(device: &str) -> Result<Vec<String>, Error> {
	Ok(get_device_profiles(device)?)
}

#[command]
pub async fn get_selected_profile(device: String) -> Result<Profile, Error> {
	let mut locks = acquire_locks_mut().await;
	if !DEVICES.contains_key(&device) {
		return Err(Error::new(format!("device {device} not found")));
	}

	let selected_profile = locks.device_stores.get_selected_profile(&device)?;
	let store = locks.profile_stores.get_profile_store_mut(&DEVICES.get(&device).unwrap(), &selected_profile).await?;
	store.value.hydrate_current_slots();

	Ok(store.value.clone())
}

#[command]
pub async fn set_selected_profile(device: String, id: String) -> Result<(), Error> {
	let mut locks = acquire_locks_mut().await;
	if !DEVICES.contains_key(&device) {
		return Err(Error::new(format!("device {device} not found")));
	}

	// If a profile save is pending for this device, save it immediately to prevent losing profile data
	if let Err(error) = save_profile_now(&device, &mut locks).await {
		log::error!("Failed to save profile for device {device}: {error}");
	}

	let selected_profile = locks.device_stores.get_selected_profile(&device)?;

	if selected_profile != id {
		let old_profile = &locks.profile_stores.get_profile_store(&DEVICES.get(&device).unwrap(), &selected_profile)?.value;
		crate::events::outbound::will_appear::disappear_page(old_profile, false).await;
		let _ = crate::events::outbound::devices::clear_screen(device.clone()).await;
	}

	// We must use the mutable version of get_profile_store in order to create the store if it does not exist.
	let store = locks.profile_stores.get_profile_store_mut(&DEVICES.get(&device).unwrap(), &id).await?;
	store.value.ensure_pages();
	store.value.hydrate_current_slots();
	crate::events::outbound::will_appear::appear_page(&store.value).await;
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

pub async fn rerender_images(app: &AppHandle) -> Result<(), anyhow::Error> {
	let window = app.get_webview_window("main").unwrap();
	window.emit("rerender_images", ())?;
	Ok(())
}

#[derive(Clone, serde::Serialize)]
struct SwitchPageEvent {
	device: String,
	profile: Profile,
}

async fn publish_page(device: &str, profile: &Profile) {
	// The webview applies slots, then calls refresh_current_page. willAppear before that
	// races the old page and the setImage is dropped.
	emit_switch_page(device, profile).await;
}

#[command]
pub async fn refresh_current_page(device: String, page: usize) -> Result<(), Error> {
	let mut locks = acquire_locks_mut().await;
	if !DEVICES.contains_key(&device) {
		return Err(Error::new(format!("device {device} not found")));
	}

	let selected_profile = locks.device_stores.get_selected_profile(&device)?;
	let store = locks.profile_stores.get_profile_store(&DEVICES.get(&device).unwrap(), &selected_profile)?;
	if store.value.current_page_index() != page {
		return Ok(());
	}
	let profile = store.value.clone();
	drop(locks);

	crate::events::outbound::will_appear::appear_page(&profile).await;
	Ok(())
}

async fn emit_switch_page(device: &str, profile: &Profile) {
	if let Some(app) = crate::APP_HANDLE.get()
		&& let Some(window) = app.get_webview_window("main")
	{
		let _ = window.emit(
			"switch_page",
			SwitchPageEvent {
				device: device.to_owned(),
				profile: profile.clone(),
			},
		);
	}
}

pub async fn set_current_page(device: &str, page: usize) -> Result<Profile, Error> {
	let mut locks = acquire_locks_mut().await;
	if !DEVICES.contains_key(device) {
		return Err(Error::new(format!("device {device} not found")));
	}

	let selected_profile = locks.device_stores.get_selected_profile(device)?;
	let store = locks.profile_stores.get_profile_store_mut(&DEVICES.get(device).unwrap(), &selected_profile).await?;
	store.value.ensure_pages();
	let page = page.min(store.value.pages.len().saturating_sub(1));
	if page == store.value.current_page_index() {
		store.value.hydrate_current_slots();
		return Ok(store.value.clone());
	}

	crate::events::outbound::will_appear::disappear_page(&store.value, false).await;
	let _ = crate::events::outbound::devices::clear_screen(device.to_owned()).await;
	store.value.current_page = page;
	store.value.hydrate_current_slots();
	store.save()?;
	let profile = store.value.clone();
	drop(locks);

	publish_page(device, &profile).await;
	Ok(profile)
}

#[command]
pub async fn set_selected_page(device: String, page: usize) -> Result<Profile, Error> {
	set_current_page(&device, page).await
}

#[command]
pub async fn add_profile_page(device: String) -> Result<Profile, Error> {
	let mut locks = acquire_locks_mut().await;
	if !DEVICES.contains_key(&device) {
		return Err(Error::new(format!("device {device} not found")));
	}

	let selected_profile = locks.device_stores.get_selected_profile(&device)?;
	let device_info = DEVICES.get(&device).unwrap();
	let store = locks.profile_stores.get_profile_store_mut(&device_info, &selected_profile).await?;
	store.value.ensure_pages();
	store.value.pages.push(ProfilePage::empty(&device_info));
	let new_page = store.value.pages.len() - 1;
	drop(locks);

	set_current_page(&device, new_page).await
}

#[command]
pub async fn delete_profile_page(device: String, page: usize) -> Result<Profile, Error> {
	let mut locks = acquire_locks_mut().await;
	if !DEVICES.contains_key(&device) {
		return Err(Error::new(format!("device {device} not found")));
	}

	let selected_profile = locks.device_stores.get_selected_profile(&device)?;
	let store = locks.profile_stores.get_profile_store_mut(&DEVICES.get(&device).unwrap(), &selected_profile).await?;
	store.value.ensure_pages();
	if store.value.pages.len() <= 1 {
		store.value.hydrate_current_slots();
		return Ok(store.value.clone());
	}
	if page >= store.value.pages.len() {
		return Err(Error::new(format!("page {page} not found")));
	}

	let current = store.value.current_page_index();
	crate::events::outbound::will_appear::disappear_page(&store.value, false).await;
	let _ = crate::events::outbound::devices::clear_screen(device.clone()).await;

	store.value.pages.remove(page);
	store.value.reindex_pages();
	store.value.current_page = if current == page {
		page.saturating_sub(1)
	} else if current > page {
		current - 1
	} else {
		current
	};
	store.value.hydrate_current_slots();
	store.save()?;
	let profile = store.value.clone();
	drop(locks);

	publish_page(&device, &profile).await;
	Ok(profile)
}

pub async fn next_page(device: &str) -> Result<Profile, Error> {
	let mut locks = acquire_locks_mut().await;
	if !DEVICES.contains_key(device) {
		return Err(Error::new(format!("device {device} not found")));
	}
	let selected_profile = locks.device_stores.get_selected_profile(device)?;
	let store = locks.profile_stores.get_profile_store(&DEVICES.get(device).unwrap(), &selected_profile)?;
	let len = store.value.pages.len().max(1);
	let current = store.value.current_page_index();
	drop(locks);
	set_current_page(device, (current + 1) % len).await
}

pub async fn previous_page(device: &str) -> Result<Profile, Error> {
	let mut locks = acquire_locks_mut().await;
	if !DEVICES.contains_key(device) {
		return Err(Error::new(format!("device {device} not found")));
	}
	let selected_profile = locks.device_stores.get_selected_profile(device)?;
	let store = locks.profile_stores.get_profile_store(&DEVICES.get(device).unwrap(), &selected_profile)?;
	let len = store.value.pages.len().max(1);
	let current = store.value.current_page_index();
	drop(locks);
	set_current_page(device, (current + len - 1) % len).await
}

pub async fn goto_page(device: &str, page: usize) -> Result<Profile, Error> {
	set_current_page(device, page.saturating_sub(1)).await
}
