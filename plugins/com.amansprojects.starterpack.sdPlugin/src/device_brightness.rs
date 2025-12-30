//! Non-spec OpenDeck-specific protocols are used in this file.

use openaction::*;

use serde::{Deserialize, Serialize};

#[derive(Serialize)]
struct DeviceBrightnessEvent {
	event: &'static str,
	action: String,
	value: u8,
}

#[derive(Serialize, Deserialize, Default, Clone)]
#[serde(default)]
pub struct DeviceBrightnessSettings {
	action: Option<String>,
	value: Option<u8>,
}

pub struct DeviceBrightnessAction;
#[async_trait]
impl Action for DeviceBrightnessAction {
	const UUID: &'static str = "com.amansprojects.starterpack.devicebrightness";
	type Settings = DeviceBrightnessSettings;

	async fn key_up(
		&self,
		_instance: &Instance,
		settings: &Self::Settings,
	) -> OpenActionResult<()> {
		send_arbitrary_json(DeviceBrightnessEvent {
			event: "deviceBrightness",
			action: settings.action.as_deref().unwrap_or("set").to_owned(),
			value: settings.value.unwrap_or(50),
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
		_instance: &Instance,
		_settings: &Self::Settings,
		ticks: i16,
		_pressed: bool,
	) -> OpenActionResult<()> {
		send_arbitrary_json(DeviceBrightnessEvent {
			event: "deviceBrightness",
			action: if ticks < 0 {
				"decrease".to_owned()
			} else {
				"increase".to_owned()
			},
			value: ticks.unsigned_abs() as u8,
		})
		.await
	}
}
