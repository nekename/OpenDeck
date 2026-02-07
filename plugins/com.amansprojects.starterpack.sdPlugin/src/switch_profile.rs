//! Non-spec OpenDeck-specific protocols are used in this file.

use openaction::*;

use serde::{Deserialize, Serialize};

#[derive(Serialize)]
struct SwitchProfileEvent {
	event: &'static str,
	device: String,
	profile: String,
}

#[derive(Serialize, Deserialize, Default, Clone)]
#[serde(default)]
pub struct SwitchProfileSettings {
	device: Option<String>,
	profile: Option<String>,
	anticlockwise: Option<String>,
	clockwise: Option<String>,
}

pub struct SwitchProfileAction;
#[async_trait]
impl Action for SwitchProfileAction {
	const UUID: &'static str = "com.amansprojects.starterpack.switchprofile";
	type Settings = SwitchProfileSettings;

	async fn key_up(&self, instance: &Instance, settings: &Self::Settings) -> OpenActionResult<()> {
		send_arbitrary_json(SwitchProfileEvent {
			event: "switchProfile",
			device: settings
				.device
				.as_deref()
				.unwrap_or(&instance.device_id)
				.to_owned(),
			profile: settings.profile.as_deref().unwrap_or("Default").to_owned(),
		})
		.await
	}

	async fn dial_up(
		&self,
		instance: &Instance,
		settings: &Self::Settings,
	) -> OpenActionResult<()> {
		self.key_up(instance, settings).await
	}

	async fn dial_rotate(
		&self,
		instance: &Instance,
		settings: &Self::Settings,
		ticks: i16,
		_pressed: bool,
	) -> OpenActionResult<()> {
		let profile = if ticks < 0 {
			&settings.anticlockwise
		} else {
			&settings.clockwise
		};
		send_arbitrary_json(SwitchProfileEvent {
			event: "switchProfile",
			device: settings
				.device
				.as_deref()
				.unwrap_or(&instance.device_id)
				.to_owned(),
			profile: profile.as_deref().unwrap_or("Default").to_owned(),
		})
		.await
	}

	async fn property_inspector_did_appear(
		&self,
		instance: &Instance,
		_settings: &Self::Settings,
	) -> OpenActionResult<()> {
		instance
			.send_to_property_inspector(serde_json::json!({
				"event": "updateDevices",
				"devices": get_connected_devices().await.keys().collect::<Vec<_>>(),
			}))
			.await
	}
}

pub(crate) async fn update_devices() -> OpenActionResult<()> {
	let message = serde_json::json!({
		"event": "updateDevices",
		"devices": get_connected_devices().await.keys().collect::<Vec<_>>(),
	});
	for instance in visible_instances(SwitchProfileAction::UUID).await {
		instance.send_to_property_inspector(message.clone()).await?;
	}
	Ok(())
}
