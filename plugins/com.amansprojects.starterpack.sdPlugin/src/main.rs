mod device_brightness;
mod input_simulation;
mod run_command;
mod switch_profile;

use openaction::*;

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

	register_action(device_brightness::DeviceBrightnessAction).await;
	register_action(input_simulation::InputSimulationAction).await;
	register_action(run_command::RunCommandAction).await;
	register_action(switch_profile::SwitchProfileAction).await;

	run(std::env::args().collect()).await
}
