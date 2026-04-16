use super::PayloadEvent;

use crate::plugins::DEVICE_NAMESPACES;
use crate::shared::DEVICES;
use crate::store::profiles::get_device_profiles;

use serde::Deserialize;
use tauri::{Emitter, Manager};

pub async fn register_device(uuid: &str, mut event: PayloadEvent<crate::shared::DeviceInfo>) -> Result<(), anyhow::Error> {
	if uuid.is_empty() || Some(uuid) == DEVICE_NAMESPACES.read().await.get(&event.payload.id[..2]).map(|x| x.as_str()) {
		if let Ok(profiles) = get_device_profiles(&event.payload.id) {
			let mut profile_stores = crate::store::profiles::PROFILE_STORES.write().await;
			for profile in profiles {
				// This is called to initialise the store for each profile when the device is registered.
				if let Err(e) = profile_stores.get_profile_store_mut(&event.payload, &profile).await {
					log::error!("{}", e);
				}
			}
		}

		event.payload.plugin = uuid.to_owned();
		let _ = crate::events::outbound::devices::device_did_connect(&event.payload.id, (&event.payload).into()).await;
		DEVICES.insert(event.payload.id.clone(), event.payload.clone());
		let _ = crate::device_sleep::note_activity(&event.payload.id).await;
		crate::events::frontend::update_devices().await;

		let mut locks = crate::store::profiles::acquire_locks_mut().await;
		let selected_profile = locks.device_stores.get_selected_profile(&event.payload.id)?;
		let profile = locks.profile_stores.get_profile_store(&DEVICES.get(&event.payload.id).unwrap(), &selected_profile)?;
		for instance in profile.value.keys.iter().flatten().chain(profile.value.sliders.iter().flatten()) {
			let _ = crate::events::outbound::will_appear::will_appear(instance).await;
		}

		use tauri_plugin_aptabase::EventTracker;
		let _ = crate::APP_HANDLE
			.get()
			.unwrap()
			.track_event("device_registered", Some(serde_json::json!({ "name": event.payload.name })));

		Ok(())
	} else {
		Err(anyhow::anyhow!("plugin {uuid} is not registered for device namespace {}", &event.payload.id[..2]))
	}
}

pub async fn deregister_device(uuid: &str, event: PayloadEvent<String>) -> Result<(), anyhow::Error> {
	if uuid.is_empty() || Some(uuid) == DEVICE_NAMESPACES.read().await.get(&event.payload[..2]).map(|x| x.as_str()) {
		if !DEVICES.contains_key(&event.payload) {
			return Ok(());
		}

		let mut locks = crate::store::profiles::acquire_locks_mut().await;

		let selected_profile = locks.device_stores.get_selected_profile(&event.payload)?;
		let profile = locks.profile_stores.get_profile_store(&DEVICES.get(&event.payload).unwrap(), &selected_profile)?;
		for instance in profile.value.keys.iter().flatten().chain(profile.value.sliders.iter().flatten()) {
			let _ = crate::events::outbound::will_appear::will_disappear(instance, false).await;
		}

		if let Ok(profiles) = get_device_profiles(&event.payload) {
			for profile in profiles {
				locks.profile_stores.remove_profile(&event.payload, &profile);
			}
		}

		drop(locks);

		let _ = crate::events::outbound::devices::device_did_disconnect(&event.payload).await;
		DEVICES.remove(&event.payload);
		crate::device_sleep::deregister_device(&event.payload);
		crate::events::frontend::update_devices().await;

		Ok(())
	} else {
		Err(anyhow::anyhow!("plugin {uuid} is not registered for device namespace {}", &event.payload[..2]))
	}
}

#[derive(Deserialize)]
pub struct PressPayload {
	pub device: String,
	pub position: u8,
}

pub async fn key_down(event: PayloadEvent<PressPayload>) -> Result<(), anyhow::Error> {
	if crate::device_sleep::note_activity(&event.payload.device).await.unwrap_or(false) {
		return Ok(());
	}
	crate::events::outbound::keypad::key_down(&event.payload.device, event.payload.position).await
}

pub async fn key_up(event: PayloadEvent<PressPayload>) -> Result<(), anyhow::Error> {
	if crate::device_sleep::note_activity(&event.payload.device).await.unwrap_or(false) {
		return Ok(());
	}
	crate::events::outbound::keypad::key_up(&event.payload.device, event.payload.position).await
}

#[derive(Deserialize)]
pub struct TicksPayload {
	pub device: String,
	pub position: u8,
	pub ticks: i16,
}

pub async fn encoder_change(event: PayloadEvent<TicksPayload>) -> Result<(), anyhow::Error> {
	if crate::device_sleep::note_activity(&event.payload.device).await.unwrap_or(false) {
		return Ok(());
	}
	crate::events::outbound::encoder::dial_rotate(&event.payload.device, event.payload.position, event.payload.ticks).await
}

pub async fn encoder_down(event: PayloadEvent<PressPayload>) -> Result<(), anyhow::Error> {
	if crate::device_sleep::note_activity(&event.payload.device).await.unwrap_or(false) {
		return Ok(());
	}
	crate::events::outbound::encoder::dial_press(&event.payload.device, "dialDown", event.payload.position).await
}

pub async fn encoder_up(event: PayloadEvent<PressPayload>) -> Result<(), anyhow::Error> {
	if crate::device_sleep::note_activity(&event.payload.device).await.unwrap_or(false) {
		return Ok(());
	}
	crate::events::outbound::encoder::dial_press(&event.payload.device, "dialUp", event.payload.position).await
}

#[derive(Deserialize)]
pub struct TouchPayload {
	pub device: String,
	pub position: u8,
	pub tap_pos: (u16, u16),
	pub hold: bool,
}

pub async fn touch_tap(event: PayloadEvent<TouchPayload>) -> Result<(), anyhow::Error> {
	if crate::device_sleep::note_activity(&event.payload.device).await.unwrap_or(false) {
		return Ok(());
	}
	crate::events::outbound::encoder::touch_tap(&event.payload.device, event.payload.position, event.payload.tap_pos, event.payload.hold).await
}

/// Handle a touch-strip swipe gesture detected by the HID layer.
/// Any horizontal swipe in either direction triggers a profile switch:
/// swipe right = next profile alphabetically, swipe left = previous.
/// Uses the full profile switch path (willDisappear / clear_screen /
/// willAppear) so plugins are properly notified.
pub async fn touch_swipe(device: String, from: (u16, u16), to: (u16, u16)) -> Result<(), anyhow::Error> {
	if crate::device_sleep::note_activity(&device).await.unwrap_or(false) {
		return Ok(());
	}
	let dx = to.0 as i32 - from.0 as i32;
	if dx == 0 {
		return Ok(());
	}
	let direction: i8 = if dx > 0 { 1 } else { -1 };

	let mut locks = crate::store::profiles::acquire_locks_mut().await;
	let current = locks.device_stores.get_selected_profile(&device)?;
	let device_info = crate::shared::DEVICES.get(&device).ok_or_else(|| anyhow::anyhow!("device not found"))?;
	let profile_store = locks.profile_stores.get_profile_store(&device_info, &current)?;

	// Carousel convention: swiping right (dx > 0) reveals the profile
	// to the LEFT (like scrolling a page), and vice versa.
	let configured = if direction > 0 {
		profile_store.value.swipe_left.clone()
	} else {
		profile_store.value.swipe_right.clone()
	};
	drop(locks);

	let next_profile = if let Some(configured) = configured {
		configured
	} else {
		let profile_dir = crate::shared::config_dir().join("profiles").join(&device);
		let mut profiles: Vec<String> = std::fs::read_dir(&profile_dir)?
			.filter_map(|e| e.ok())
			.filter_map(|e| {
				let name = e.file_name().to_string_lossy().to_string();
				if name.ends_with(".json") && !name.contains("backup") {
					Some(name.trim_end_matches(".json").to_owned())
				} else {
					None
				}
			})
			.collect();
		profiles.sort();
		if profiles.len() < 2 {
			return Ok(());
		}
		let current_idx = profiles.iter().position(|p| p == &current).unwrap_or(0);
		// Swipe right (direction > 0) = previous (carousel: reveal left)
		let next_idx = if direction > 0 {
			(current_idx + profiles.len() - 1) % profiles.len()
		} else {
			(current_idx + 1) % profiles.len()
		};
		profiles[next_idx].clone()
	};
	log::info!("Swipe {} on {}: {} -> {}", if direction > 0 { "right" } else { "left" }, device, current, next_profile);

	// Use the full profile switch function which fires willDisappear for
	// old slots, clears the device screen, and fires willAppear for new
	// slots. This is the same path the frontend UI uses.
	crate::events::frontend::profiles::set_selected_profile(device.clone(), next_profile.clone()).await?;

	let app_handle = crate::APP_HANDLE.get().unwrap();
	app_handle.get_webview_window("main").unwrap().emit(
		"switch_profile",
		crate::events::inbound::misc::SwitchProfileEvent::new(device, next_profile),
	)?;
	Ok(())
}

pub async fn rerender_images(_event: PayloadEvent<String>) -> Result<(), anyhow::Error> {
	crate::events::frontend::profiles::rerender_images(crate::APP_HANDLE.get().unwrap()).await?;
	Ok(())
}
