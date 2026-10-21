//! Non-spec OpenDeck-specific protocols are used in this file.

use openaction::*;

use serde::{Deserialize, Serialize};

use std::collections::HashMap;
use std::sync::LazyLock;
use std::time::Duration;

use tokio::sync::Mutex;

/// How long a key/dial must be held before it counts as a "long click" and switches
/// to the long-click profile immediately, without waiting for release.
const LONG_PRESS_THRESHOLD: Duration = Duration::from_millis(600);

#[derive(Default)]
struct PressState {
	/// Incremented on every key/dial down so a stale timer from an earlier press can
	/// recognise that it no longer applies.
	generation: u64,
	/// Whether the key/dial is currently held down.
	active: bool,
	/// Whether the long-click profile switch already fired for the current press.
	long_fired: bool,
}

static PRESS_STATE: LazyLock<Mutex<HashMap<InstanceId, PressState>>> =
	LazyLock::new(|| Mutex::new(HashMap::new()));

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
	#[serde(rename = "longProfile")]
	long_profile: Option<String>,
	anticlockwise: Option<String>,
	clockwise: Option<String>,
}

pub struct SwitchProfileAction;
#[async_trait]
impl Action for SwitchProfileAction {
	const UUID: &'static str = "com.amansprojects.starterpack.switchprofile";
	type Settings = SwitchProfileSettings;

	async fn key_down(
		&self,
		instance: &Instance,
		settings: &Self::Settings,
	) -> OpenActionResult<()> {
		// Only bother scheduling a timer if a long-click profile is actually configured;
		// otherwise behaviour should be identical to a plugin with no long-click support.
		let Some(long_profile) = settings
			.long_profile
			.as_deref()
			.filter(|profile| !profile.trim().is_empty())
			.map(|profile| profile.to_owned())
		else {
			return Ok(());
		};

		let instance_id = instance.instance_id.clone();
		let generation = {
			let mut states = PRESS_STATE.lock().await;
			let state = states.entry(instance_id.clone()).or_default();
			state.generation += 1;
			state.active = true;
			state.long_fired = false;
			state.generation
		};

		let device = settings
			.device
			.as_deref()
			.unwrap_or(&instance.device_id)
			.to_owned();

		tokio::spawn(async move {
			tokio::time::sleep(LONG_PRESS_THRESHOLD).await;

			let mut states = PRESS_STATE.lock().await;
			let Some(state) = states.get_mut(&instance_id) else {
				return;
			};
			// Bail out if a newer press has started, the key was already released, or
			// the long-click switch already fired for this press.
			if state.generation != generation || !state.active || state.long_fired {
				return;
			}
			state.long_fired = true;
			drop(states);

			if let Err(error) = send_arbitrary_json(SwitchProfileEvent {
				event: "switchProfile",
				device,
				profile: long_profile,
			})
			.await
			{
				log::warn!("Failed to switch profile on long press: {error}");
			}
		});

		Ok(())
	}

	async fn key_up(&self, instance: &Instance, settings: &Self::Settings) -> OpenActionResult<()> {
		let long_already_fired = {
			let mut states = PRESS_STATE.lock().await;
			match states.get_mut(&instance.instance_id) {
				Some(state) => {
					state.active = false;
					state.long_fired
				}
				None => false,
			}
		};

		if long_already_fired {
			return Ok(());
		}

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

	async fn dial_down(
		&self,
		instance: &Instance,
		settings: &Self::Settings,
	) -> OpenActionResult<()> {
		self.key_down(instance, settings).await
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
