use super::PayloadEvent;

use crate::plugins::DEVICE_NAMESPACES;
use crate::shared::DEVICES;
use crate::store::profiles::get_device_profiles;

use serde::Deserialize;

pub async fn register_device(uuid: &str, mut event: PayloadEvent<crate::shared::DeviceInfo>) -> Result<(), anyhow::Error> {
	if let Some(background) = &event.payload.background
		&& !crate::shared::is_valid_background(background)
	{
		log::warn!("Plugin {uuid} provided an invalid background image for device {}", event.payload.id);
		event.payload.background = None;
	}

	sanitize_slots(uuid, &mut event.payload);

	// The background is kept out of the device itself.
	let background = event.payload.background.take();

	if uuid.is_empty() || Some(uuid) == DEVICE_NAMESPACES.read().await.get(namespace(&event.payload.id).unwrap_or_default()).map(|x| x.as_str()) {
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
		if let Some(background) = &background {
			crate::shared::DEVICE_BACKGROUNDS.insert(event.payload.id.clone(), background.clone());
		}
		crate::events::frontend::device_background(&event.payload.id, background.clone());
		let _ = crate::events::outbound::devices::set_device_brightness(&event.payload.id, crate::store::get_settings().value.brightness).await;
		let _ = crate::device_sleep::apply_initial_device_sleep(&event.payload.id).await;
		crate::events::frontend::update_devices().await;

		let mut locks = crate::store::profiles::acquire_locks_mut().await;
		let selected_profile = locks.device_stores.get_selected_profile(&event.payload.id)?;
		let profile = locks.profile_stores.get_profile_store(&DEVICES.get(&event.payload.id).unwrap(), &selected_profile)?;
		for instance in profile
			.value
			.keys
			.iter()
			.flatten()
			.chain(profile.value.sliders.iter().flatten())
			.chain(profile.value.infobars.iter().flatten())
		{
			let _ = crate::events::outbound::will_appear::will_appear(instance).await;
		}

		use tauri_plugin_aptabase::EventTracker;
		let _ = crate::APP_HANDLE
			.get()
			.unwrap()
			.track_event("device_registered", Some(serde_json::json!({ "name": event.payload.name })));

		Ok(())
	} else {
		Err(anyhow::anyhow!(
			"plugin {uuid} is not registered for device namespace {}",
			namespace(&event.payload.id).unwrap_or_default()
		))
	}
}

pub async fn deregister_device(uuid: &str, event: PayloadEvent<String>) -> Result<(), anyhow::Error> {
	if uuid.is_empty() || Some(uuid) == DEVICE_NAMESPACES.read().await.get(namespace(&event.payload).unwrap_or_default()).map(|x| x.as_str()) {
		if !DEVICES.contains_key(&event.payload) {
			return Ok(());
		}

		let mut locks = crate::store::profiles::acquire_locks_mut().await;

		let selected_profile = locks.device_stores.get_selected_profile(&event.payload)?;
		let profile = locks.profile_stores.get_profile_store(&DEVICES.get(&event.payload).unwrap(), &selected_profile)?;
		for instance in profile
			.value
			.keys
			.iter()
			.flatten()
			.chain(profile.value.sliders.iter().flatten())
			.chain(profile.value.infobars.iter().flatten())
		{
			let _ = crate::events::outbound::will_appear::will_disappear(instance, false).await;
		}

		// Flush any pending profile writes before removing the device.
		if let Err(error) = crate::store::profiles::save_profile_now(&event.payload, &mut locks).await {
			log::error!("Failed to flush profile for device {}: {error}", event.payload);
		}

		if let Ok(profiles) = get_device_profiles(&event.payload) {
			for profile in profiles {
				locks.profile_stores.remove_profile(&event.payload, &profile);
			}
		}

		drop(locks);

		crate::shared::DEVICE_BACKGROUNDS.remove(&event.payload);
		crate::events::frontend::device_background(&event.payload, None);

		let _ = crate::events::outbound::devices::device_did_disconnect(&event.payload).await;
		DEVICES.remove(&event.payload);
		crate::device_sleep::deregister_device(&event.payload);
		crate::events::frontend::update_devices().await;

		Ok(())
	} else {
		Err(anyhow::anyhow!(
			"plugin {uuid} is not registered for device namespace {}",
			namespace(&event.payload).unwrap_or_default()
		))
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

/// The namespace of a device: the two characters its id starts with, which the
/// plugin declares in its manifest.
fn namespace(device: &str) -> Option<&str> {
	device.get(..2)
}

fn sanitize_slots(uuid: &str, device: &mut crate::shared::DeviceInfo) {
	let id = device.id.clone();
	let cells = device.rows as usize * device.columns as usize;
	let encoder_count = device.encoders;

	let Some(slots) = device.slots.as_mut() else {
		return;
	};

	let mut positions = std::collections::HashSet::new();
	let mut encoders = std::collections::HashSet::new();

	slots.retain_mut(|slot| {
		if cells == 0 || slot.position as usize >= cells {
			log::warn!("Plugin {uuid} gave device {id} a slot at position {}, which is outside of its grid", slot.position);
			return false;
		}

		if !positions.insert(slot.position) {
			log::warn!("Plugin {uuid} gave device {id} more than one slot at position {}", slot.position);
			return false;
		}

		if let Some(encoder) = slot.encoder {
			if encoder >= encoder_count {
				log::warn!("Plugin {uuid} gave device {id} a slot for encoder {}, which it does not have", encoder);
				slot.encoder = None;
			} else if !encoders.insert(encoder) {
				log::warn!("Plugin {uuid} gave device {id} more than one slot for encoder {}", encoder);
				slot.encoder = None;
			}
		}

		true
	});
}

/// Payload for `setDeviceBackground`.
#[derive(Deserialize)]
pub struct DeviceBackgroundPayload {
	pub device: String,
	#[serde(default)]
	pub background: Option<String>,
}

/// Set (or clear) the background image of a device. Only the plugin that owns
/// the device may change it, and it is not persisted.
pub async fn set_device_background(uuid: &str, event: PayloadEvent<DeviceBackgroundPayload>) -> Result<(), anyhow::Error> {
	if !owns_device(uuid, &event.payload.device).await {
		return Err(anyhow::anyhow!("plugin {uuid} does not own device {}", event.payload.device));
	}

	if let Some(background) = &event.payload.background
		&& !crate::shared::is_valid_background(background)
	{
		return Err(anyhow::anyhow!("plugin {uuid} provided an invalid background image for device {}", event.payload.device));
	}

	let background = event.payload.background.clone();

	if !DEVICES.contains_key(&event.payload.device) {
		return Ok(());
	}

	match &background {
		Some(background) => {
			crate::shared::DEVICE_BACKGROUNDS.insert(event.payload.device.clone(), background.clone());
		}
		None => {
			crate::shared::DEVICE_BACKGROUNDS.remove(&event.payload.device);
		}
	}

	crate::events::frontend::device_background(&event.payload.device, background);

	Ok(())
}

/// Whether the plugin is the one that registered the namespace the device
/// belongs to.
pub async fn owns_device(uuid: &str, device: &str) -> bool {
	if uuid.is_empty() {
		return false;
	}
	let Some(namespace) = device.get(..2) else {
		return false;
	};
	Some(uuid) == DEVICE_NAMESPACES.read().await.get(namespace).map(|x| x.as_str())
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
pub struct TouchscreenPressPayload {
	pub device: String,
	pub position: u8,
	pub x: u16,
	pub y: u16,
	#[serde(default)]
	pub hold: bool,
}

pub async fn touchscreen_press(event: PayloadEvent<TouchscreenPressPayload>) -> Result<(), anyhow::Error> {
	if crate::device_sleep::note_activity(&event.payload.device).await.unwrap_or(false) {
		return Ok(());
	}
	crate::events::outbound::encoder::touch_tap(&event.payload.device, event.payload.position, event.payload.x, event.payload.y, event.payload.hold).await
}

pub async fn rerender_images(_event: PayloadEvent<String>) -> Result<(), anyhow::Error> {
	crate::events::frontend::profiles::rerender_images(crate::APP_HANDLE.get().unwrap()).await?;
	Ok(())
}

#[cfg(test)]
mod tests {
	use super::*;

	fn device(slots: Vec<crate::shared::DeviceSlot>) -> crate::shared::DeviceInfo {
		crate::shared::DeviceInfo {
			id: "ab".to_owned(),
			plugin: String::new(),
			name: "Test".to_owned(),
			rows: 2,
			columns: 3,
			encoders: 2,
			touchpoints: 0,
			infobars: 0,
			r#type: 0,
			background: None,
			encoder_alignment: None,
			slots: Some(slots),
		}
	}

	fn slot(position: u8, encoder: Option<u8>) -> crate::shared::DeviceSlot {
		crate::shared::DeviceSlot { position, round: false, encoder }
	}

	#[test]
	fn slots_outside_of_the_grid_are_dropped() {
		let mut device = device(vec![slot(6, None), slot(7, None)]);
		sanitize_slots("plugin", &mut device);
		assert_eq!(device.slots.as_ref().unwrap().len(), 0);
	}

	#[test]
	fn duplicate_slots_are_dropped() {
		let mut device = device(vec![slot(1, None), slot(1, None)]);
		sanitize_slots("plugin", &mut device);
		assert_eq!(device.slots.as_ref().unwrap().len(), 1);
	}

	#[test]
	fn encoders_the_device_does_not_have_are_dropped() {
		let mut device = device(vec![slot(1, Some(5)), slot(2, Some(0)), slot(3, Some(0))]);
		sanitize_slots("plugin", &mut device);
		let slots = device.slots.as_ref().unwrap();
		assert_eq!(slots[0].encoder, None);
		assert_eq!(slots[1].encoder, Some(0));
		assert_eq!(slots[2].encoder, None);
	}

	#[test]
	fn a_slot_at_the_last_position_of_the_grid_is_kept() {
		let mut device = device(vec![slot(5, Some(1))]);
		sanitize_slots("plugin", &mut device);
		let slots = device.slots.as_ref().unwrap();
		assert_eq!(slots.len(), 1);
		assert_eq!(slots[0].position, 5);
		assert_eq!(slots[0].encoder, Some(1));
	}

	#[test]
	fn a_device_without_a_grid_has_no_slots() {
		let mut device = device(vec![slot(0, None)]);
		device.rows = 0;
		sanitize_slots("plugin", &mut device);
		assert!(device.slots.as_ref().unwrap().is_empty());
	}

	#[test]
	fn a_slot_keeps_its_shape() {
		let mut device = device(vec![crate::shared::DeviceSlot {
			position: 3,
			round: true,
			encoder: None,
		}]);
		sanitize_slots("plugin", &mut device);
		let slots = device.slots.as_ref().unwrap();
		assert_eq!(slots.len(), 1);
		assert!(slots[0].round);
	}

	#[test]
	fn the_namespace_is_the_first_two_characters_of_the_id() {
		assert_eq!(namespace("ex-0123456789ab"), Some("ex"));
		assert_eq!(namespace("ab"), Some("ab"));
		assert_eq!(namespace("a"), None);
		assert_eq!(namespace(""), None);
	}

	#[tokio::test]
	async fn only_the_plugin_of_a_namespace_owns_its_devices() {
		crate::plugins::DEVICE_NAMESPACES.write().await.insert("zz".to_owned(), "plugin".to_owned());

		assert!(owns_device("plugin", "zz-0123456789ab").await);
		assert!(!owns_device("other", "zz-0123456789ab").await);
		assert!(!owns_device("", "zz-0123456789ab").await);
		assert!(!owns_device("plugin", "ex-0123456789ab").await);
		assert!(!owns_device("plugin", "z").await);
	}

	/// The payload a plugin sends when it registers a device.
	fn registration(extra: serde_json::Value) -> serde_json::Value {
		let mut payload = serde_json::json!({
			"id": "ex-0123456789ab",
			"name": "Example Controller",
			"rows": 4,
			"columns": 5,
			"encoders": 1,
			"type": 0
		});
		for (key, value) in extra.as_object().unwrap() {
			payload[key] = value.clone();
		}
		payload
	}

	#[test]
	fn a_registration_without_a_decoration_is_accepted() {
		let device: crate::shared::DeviceInfo = serde_json::from_value(registration(serde_json::json!({}))).unwrap();
		assert_eq!(device.id, "ex-0123456789ab");
		assert_eq!(device.background, None);
		assert_eq!(device.encoder_alignment, None);
		assert_eq!(device.slots, None);
	}

	#[test]
	fn a_registration_carries_its_decoration() {
		let device: crate::shared::DeviceInfo = serde_json::from_value(registration(serde_json::json!({
			"background": "data:image/png;base64,AAAA",
			"encoder_alignment": "center",
			"slots": [{ "position": 12, "round": true, "encoder": 0 }, { "position": 1 }]
		})))
		.unwrap();

		assert_eq!(device.background.as_deref(), Some("data:image/png;base64,AAAA"));
		assert_eq!(device.encoder_alignment, Some(crate::shared::EncoderAlignment::Center));

		let slots = device.slots.as_ref().unwrap();
		assert_eq!(slots.len(), 2);
		assert_eq!(slots[0].position, 12);
		assert!(slots[0].round);
		assert_eq!(slots[0].encoder, Some(0));
		assert_eq!(slots[1].position, 1);
		assert!(!slots[1].round);
		assert_eq!(slots[1].encoder, None);
	}
}
