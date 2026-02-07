mod device_brightness;
mod input_simulation;
mod open_url;
mod run_command;
mod switch_profile;

use openaction::*;

struct GlobalEventHandler;
#[async_trait]
impl global_events::GlobalEventHandler for GlobalEventHandler {
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
