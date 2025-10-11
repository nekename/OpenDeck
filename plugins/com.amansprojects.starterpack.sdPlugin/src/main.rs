mod device_brightness;
mod input_simulation;
mod run_command;
mod switch_profile;

use openaction::*;

trait ActionEvent {
	fn context(&self) -> &String;
	fn settings(&self) -> &SettingsValue;
}
impl ActionEvent for KeyEvent {
	fn context(&self) -> &String {
		&self.context
	}
	fn settings(&self) -> &SettingsValue {
		&self.payload.settings
	}
}
impl ActionEvent for DialPressEvent {
	fn context(&self) -> &String {
		&self.context
	}
	fn settings(&self) -> &SettingsValue {
		&self.payload.settings
	}
}
impl ActionEvent for DialRotateEvent {
	fn context(&self) -> &String {
		&self.context
	}
	fn settings(&self) -> &SettingsValue {
		&self.payload.settings
	}
}

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
			"com.amansprojects.starterpack.runcommand" => run_command::down_up("down", event),
			"com.amansprojects.starterpack.inputsimulation" => {
				input_simulation::down_up("down", event).await
			}
			_ => Ok(()),
		}
	}

	async fn key_up(
		&self,
		event: KeyEvent,
		outbound: &mut openaction::OutboundEventManager,
	) -> EventHandlerResult {
		match &event.action[..] {
			"com.amansprojects.starterpack.runcommand" => run_command::down_up("up", event),
			"com.amansprojects.starterpack.inputsimulation" => {
				input_simulation::down_up("up", event).await
			}
			"com.amansprojects.starterpack.switchprofile" => {
				switch_profile::key_up(event, outbound).await
			}
			"com.amansprojects.starterpack.devicebrightness" => {
				device_brightness::up(event, outbound).await
			}
			_ => Ok(()),
		}
	}

	async fn dial_down(
		&self,
		event: DialPressEvent,
		_outbound: &mut openaction::OutboundEventManager,
	) -> EventHandlerResult {
		match &event.action[..] {
			"com.amansprojects.starterpack.runcommand" => run_command::down_up("down", event),
			"com.amansprojects.starterpack.inputsimulation" => {
				input_simulation::down_up("down", event).await
			}
			_ => Ok(()),
		}
	}

	async fn dial_up(
		&self,
		event: DialPressEvent,
		outbound: &mut openaction::OutboundEventManager,
	) -> EventHandlerResult {
		match &event.action[..] {
			"com.amansprojects.starterpack.runcommand" => run_command::down_up("up", event),
			"com.amansprojects.starterpack.inputsimulation" => {
				input_simulation::down_up("up", event).await
			}
			"com.amansprojects.starterpack.devicebrightness" => {
				device_brightness::up(event, outbound).await
			}
			_ => Ok(()),
		}
	}

	async fn dial_rotate(
		&self,
		event: DialRotateEvent,
		outbound: &mut openaction::OutboundEventManager,
	) -> EventHandlerResult {
		match &event.action[..] {
			"com.amansprojects.starterpack.runcommand" => run_command::rotate(event),
			"com.amansprojects.starterpack.inputsimulation" => {
				input_simulation::rotate(event).await
			}
			"com.amansprojects.starterpack.devicebrightness" => {
				device_brightness::rotate(event, outbound).await
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
