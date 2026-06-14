use super::ContextAndPayloadEvent;
use crate::events::frontend::instances::update_state;
use crate::store::profiles::{acquire_locks_mut, debounce_profile_save, get_instance_mut, save_profile};
use log::{debug, warn};

use serde::Deserialize;
use serde_json::Value;
use crate::shared::{config_dir, load_encoder_layout};

#[derive(Deserialize)]
pub struct SetTitlePayload {
	title: Option<String>,
	state: Option<u16>,
}

#[derive(Deserialize)]
pub struct SetImagePayload {
	image: Option<String>,
	state: Option<u16>,
}

#[derive(Deserialize)]
pub struct SetStatePayload {
	state: u16,
}

#[derive(Debug, Deserialize)]
pub struct SetFeedbackLayoutPayload {
	layout: String,
}


pub async fn set_title(event: ContextAndPayloadEvent<SetTitlePayload>) -> Result<(), anyhow::Error> {
	let mut locks = acquire_locks_mut().await;

	if let Some(instance) = get_instance_mut(&event.context, &mut locks).await? {
		if let Some(state) = event.payload.state {
			if state as usize >= instance.states.len() {
				return Err(anyhow::anyhow!("State index out of bounds ({} > {})", state, instance.states.len() - 1));
			}

			let text = event.payload.title.unwrap_or(instance.action.states[state as usize].text.clone());
			if instance.states[state as usize].text == text {
				return Ok(());
			}
			instance.states[state as usize].text = text;
		} else {
			if instance
				.states
				.iter()
				.enumerate()
				.all(|(index, state)| state.text == event.payload.title.clone().unwrap_or(instance.action.states[index].text.clone()))
			{
				return Ok(());
			}

			for (index, state) in instance.states.iter_mut().enumerate() {
				state.text = event.payload.title.clone().unwrap_or(instance.action.states[index].text.clone());
			}
		}
		update_state(crate::APP_HANDLE.get().unwrap(), instance.context.clone(), &mut locks).await?;
	}
	save_profile(&event.context.device, &mut locks).await?;

	Ok(())
}

pub async fn set_image(mut event: ContextAndPayloadEvent<SetImagePayload>) -> Result<(), anyhow::Error> {
	let mut locks = acquire_locks_mut().await;

	if let Some(instance) = get_instance_mut(&event.context, &mut locks).await? {
		if let Some(image) = &event.payload.image {
			if image.trim().is_empty() {
				event.payload.image = None;
			} else if !image.trim().starts_with("data:") {
				event.payload.image = Some(crate::shared::convert_icon(
					crate::shared::config_dir()
						.join("plugins")
						.join(&instance.action.plugin)
						.join(image.trim())
						.to_str()
						.unwrap()
						.to_owned(),
				));
			}
		}

		if let Some(state) = event.payload.state {
			if state as usize >= instance.states.len() {
				return Err(anyhow::anyhow!("State index out of bounds ({} > {})", state, instance.states.len() - 1));
			}
			instance.states[state as usize].image = event.payload.image.clone().unwrap_or(instance.action.states[state as usize].image.clone());
		} else {
			for (index, state) in instance.states.iter_mut().enumerate() {
				state.image = event.payload.image.clone().unwrap_or(instance.action.states[index].image.clone());
			}
		}
		update_state(crate::APP_HANDLE.get().unwrap(), instance.context.clone(), &mut locks).await?;
	}

	if let Some(image) = &event.payload.image
		&& image.trim().starts_with("data:")
	{
		debounce_profile_save(event.context);
	} else {
		save_profile(&event.context.device, &mut locks).await?;
	}

	Ok(())
}

pub async fn set_feedback(event: ContextAndPayloadEvent<Value>) -> Result<(), anyhow::Error> {
	// Set feedback is an object, with a list of key / value pairs
	let mut locks = acquire_locks_mut().await;
	if let Some(action) = get_instance_mut(&event.context, &mut locks).await?
		&& let Some(encoder) = &mut action.action.encoder
	{
		// We need to go through the parsed layout and update / insert values
		let layout = &mut encoder.layout_parsed;

		debug!("setFeedback: incoming: {:#?}", event.payload);
		debug!("setFeedback: layout: {:#?}", layout);

		if let Value::Object(map) = event.payload {
			let Some(items_array) = layout.get_mut("items").and_then(Value::as_array_mut) else {
				warn!("Layout has no items array");
				return Ok(());
			};

			for (key, value) in &map {
				// Locate the item by key
				let matching_item = items_array.iter_mut().find(|item| item.get("key").and_then(Value::as_str).is_some_and(|k| k == key));
				let Some(item) = matching_item else {
					warn!("setFeedback: no layout item found for key '{key}'");
					continue;
				};

				// The value is dependent on the item type, so handle that here
				item["value"] = match item.get("type").and_then(Value::as_str) {
					Some("text") => match value {
						Value::Number(n) => Value::String(n.to_string()),
						Value::String(_) => value.clone(),
						_ => {
							warn!("setFeedback: key '{key}' is a text item but received unexpected value type: {value}");
							continue;
						}
					},
					Some("pixmap") => {
						if !matches!(value, Value::String(_)) {
							warn!("setFeedback: key '{key}' is a pixmap item but received non-string value: {value}");
							continue;
						}
						value.clone()
					}
					Some("bar") | Some("gbar") => {
						if !matches!(value, Value::Number(_)) {
							warn!("setFeedback: key '{key}' is a bar item but received non-numeric value: {value}");
							continue;
						}
						value.clone()
					}
					Some(unknown) => {
						warn!("setFeedback: unknown item type '{unknown}' for key '{key}'");
						continue;
					}
					None => {
						warn!("setFeedback: item with key '{key}' has no type field");
						continue;
					}
				};
			}
		}

		// Trigger a state update, should cause a redraw
		update_state(crate::APP_HANDLE.get().unwrap(), action.context.clone(), &mut locks).await?;
	}

	Ok(())
}

pub async fn set_feedback_layout(event: ContextAndPayloadEvent<SetFeedbackLayoutPayload>) -> Result<(), anyhow::Error> {
	debug!("setFeedbackLayout: incoming: {:#?}", event.payload);

	let mut locks = acquire_locks_mut().await;
	if let Some(action) = get_instance_mut(&event.context, &mut locks).await?
		&& let Some(encoder) = &mut action.action.encoder
	{
		// We need to replace the existing parsed layout with the new one
		encoder.layout = event.payload.layout.clone();

		// Make sure the layout is a full path to the json file
		if !encoder.layout.starts_with("$") {
			let path = config_dir().join("plugins").join(&action.action.plugin);
			encoder.layout = path.join(&encoder.layout).to_string_lossy().to_string();
		}

		encoder.layout_parsed = load_encoder_layout(&encoder.layout).unwrap_or(Value::Null);

		// Trigger a state update, should cause a redraw
		update_state(crate::APP_HANDLE.get().unwrap(), action.context.clone(), &mut locks).await?;
	}
	Ok(())
}

pub async fn set_state(event: ContextAndPayloadEvent<SetStatePayload>) -> Result<(), anyhow::Error> {
	let mut locks = acquire_locks_mut().await;

	if let Some(instance) = get_instance_mut(&event.context, &mut locks).await? {
		if event.payload.state >= instance.states.len() as u16 {
			return Ok(());
		}
		instance.current_state = event.payload.state;
		update_state(crate::APP_HANDLE.get().unwrap(), instance.context.clone(), &mut locks).await?;
	}
	save_profile(&event.context.device, &mut locks).await?;

	Ok(())
}
