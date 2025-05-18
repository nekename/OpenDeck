use openaction::*;

// Non-spec OpenDeck-specific protocols are used in this file.

#[derive(serde::Serialize)]
struct DeviceBrightnessEvent {
	event: &'static str,
	action: String,
	value: u8,
}

pub async fn key_up(event: KeyEvent, outbound: &mut OutboundEventManager) -> EventHandlerResult {
	outbound
		.send_event(DeviceBrightnessEvent {
			event: "deviceBrightness",
			action: event
				.payload
				.settings
				.as_object()
				.and_then(|x| x.get("action"))
				.and_then(|x| x.as_str())
				.unwrap_or("set")
				.to_owned(),
			value: event
				.payload
				.settings
				.as_object()
				.and_then(|x| x.get("value"))
				.and_then(|x| x.as_u64())
				.unwrap_or(50) as u8,
		})
		.await?;

	Ok(())
}
