use openaction::*;

// Non-spec OpenDeck-specific protocols are used in this file.

#[derive(serde::Serialize)]
struct SwitchProfileEvent {
	event: &'static str,
	device: String,
	profile: String,
}

pub fn key_up(event: KeyEvent) -> EventHandlerResult {
	if let Some(value) = event.payload.settings.as_object().unwrap().get("profile") {
		let (mut stream, _) = tungstenite::connect(format!(
			"ws://localhost:{}",
			std::env::args().nth(2).unwrap()
		))?;
		stream.write(tungstenite::Message::text(serde_json::to_string(
			&SwitchProfileEvent {
				event: "switchProfile",
				device: event
					.payload
					.settings
					.as_object()
					.unwrap()
					.get("device")
					.unwrap()
					.as_str()
					.unwrap()
					.to_owned(),
				profile: value.as_str().unwrap().to_owned(),
			},
		)?))?;
		stream.close(None)?;
		stream.flush()?;
	}

	Ok(())
}
