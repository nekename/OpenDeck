mod device_brightness;
mod input_simulation;
mod open_url;
mod run_command;
mod switch_profile;

use std::sync::{OnceLock, RwLock};

use openaction::*;

#[derive(serde::Serialize, serde::Deserialize, Clone, Default)]
#[serde(default)]
pub struct GlobalSettings {
	#[serde(rename = "enigoRestoreToken")]
	pub enigo_restore_token: Option<String>,
}

pub fn current_settings() -> &'static RwLock<GlobalSettings> {
	static SETTINGS: OnceLock<RwLock<GlobalSettings>> = OnceLock::new();
	SETTINGS.get_or_init(|| RwLock::new(GlobalSettings::default()))
}

struct GlobalEventHandler;
#[async_trait]
impl global_events::GlobalEventHandler for GlobalEventHandler {
	async fn plugin_ready(&self) -> OpenActionResult<()> {
		get_global_settings().await
	}

	async fn did_receive_global_settings(
		&self,
		event: global_events::DidReceiveGlobalSettingsEvent,
	) -> OpenActionResult<()> {
		*current_settings().write().unwrap() =
			serde_json::from_value(event.payload.settings).unwrap_or_default();

		Ok(())
	}

	async fn device_did_connect(
		&self,
		_event: global_events::DeviceDidConnectEvent,
	) -> OpenActionResult<()> {
		switch_profile::update_devices().await
	}

	async fn device_did_disconnect(
		&self,
		_event: global_events::DeviceDidDisconnectEvent,
	) -> OpenActionResult<()> {
		switch_profile::update_devices().await
	}
}

#[tokio::main]
async fn main() -> OpenActionResult<()> {
	{
		use simplelog::*;
		if let Err(error) = TermLogger::init(
			LevelFilter::Debug,
			Config::default(),
			TerminalMode::Stdout,
			ColorChoice::Never,
		) {
			eprintln!("Logger initialization failed: {}", error);
		}
	}

	global_events::set_global_event_handler(&GlobalEventHandler);
	register_action(device_brightness::DeviceBrightnessAction).await;
	register_action(input_simulation::InputSimulationAction).await;
	register_action(open_url::OpenUrlAction).await;
	register_action(run_command::RunCommandAction).await;
	register_action(switch_profile::SwitchProfileAction).await;

	run(std::env::args().collect()).await
}
