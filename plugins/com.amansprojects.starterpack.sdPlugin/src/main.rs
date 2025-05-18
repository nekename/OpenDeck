mod device_brightness;
mod input_simulation;
mod run_command;
mod switch_profile;

use openaction::*;

struct GlobalEventHandler {}
impl openaction::GlobalEventHandler for GlobalEventHandler {}

struct ActionEventHandler {}
impl openaction::ActionEventHandler for ActionEventHandler {
	async fn key_down(
		&self,
		event: KeyEvent,
		_outbound: &mut openaction::OutboundEventManager,
	) -> EventHandlerResult {
		match &event.action[..] {
			"com.amansprojects.starterpack.runcommand" => run_command::key_down(event),
			"com.amansprojects.starterpack.inputsimulation" => input_simulation::key_down(event),
			_ => Ok(()),
		}
	}

	async fn key_up(
		&self,
		event: KeyEvent,
		outbound: &mut openaction::OutboundEventManager,
	) -> EventHandlerResult {
		match &event.action[..] {
			"com.amansprojects.starterpack.runcommand" => run_command::key_up(event),
			"com.amansprojects.starterpack.inputsimulation" => input_simulation::key_up(event),
			"com.amansprojects.starterpack.switchprofile" => {
				switch_profile::key_up(event, outbound).await
			}
			"com.amansprojects.starterpack.devicebrightness" => {
				device_brightness::key_up(event, outbound).await
			}
			_ => Ok(()),
		}
	}
}

#[tokio::main]
async fn main() {
	simplelog::TermLogger::init(
		simplelog::LevelFilter::Debug,
		simplelog::Config::default(),
		simplelog::TerminalMode::Stdout,
		simplelog::ColorChoice::Never,
	)
	.unwrap();

	if let Err(error) = init_plugin(GlobalEventHandler {}, ActionEventHandler {}).await {
		log::error!("Failed to initialise plugin: {}", error);
	}
}
