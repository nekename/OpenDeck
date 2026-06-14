use super::ContextAndPayloadEvent;
use crate::events::frontend::instances::update_state;
use crate::shared::{config_dir, load_encoder_layout};
use crate::store::profiles::{acquire_locks_mut, debounce_profile_save, get_instance_mut, save_profile};

use log::{debug, warn};

use serde::Deserialize;
use serde_json::Value;

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

const COMMON_KEYS: &[&str] = &["enabled", "opacity", "background"];
const BAR_KEYS: &[&str] = &["bar_bg_c", "bar_border_c", "bar_fill_c", "border_w", "range", "subtype", "value"];

pub async fn set_feedback(event: ContextAndPayloadEvent<Value>) -> Result<(), anyhow::Error> {
	let mut locks = acquire_locks_mut().await;

	if let Some(action) = get_instance_mut(&event.context, &mut locks).await?
		&& let Some(encoder) = &mut action.action.encoder
	{
		let layout = &mut encoder.layout_parsed;

		debug!("setFeedback: incoming: {:#?}", event.payload);
		debug!("setFeedback: layout: {:#?}", layout);

		if let Value::Object(map) = event.payload {
			let Some(items_array) = layout.get_mut("items").and_then(Value::as_array_mut) else {
				warn!("Layout has no items array");
				return Ok(());
			};

			for (key, payload_value) in &map {
				match payload_value {
					Value::String(_) | Value::Number(_) => {
						// Find matching item only (DO NOT broadcast to all items)
						let Some(item) = items_array.iter_mut().find(|item| item.get("key").and_then(Value::as_str) == Some(key)) else {
							warn!("setFeedback: no layout item found for key '{key}'");
							continue;
						};

						let item_type = item.get("type").and_then(Value::as_str);

						match item_type {
							Some("text") | Some("bar") | Some("gbar") => {
								item["value"] = match payload_value {
									Value::Number(n) => Value::Number(n.clone()),
									Value::String(s) => Value::String(s.clone()),
									_ => continue,
								};
							}

							_ => {
								// We don't need to update the value for other types
								continue;
							}
						}
					}

					Value::Object(obj) => {
						let Some(item) = items_array.iter_mut().find(|item| item.get("key").and_then(Value::as_str).is_some_and(|k| k == key)) else {
							warn!("setFeedback: no layout item found for key '{key}'");
							continue;
						};

						let type_keys: Vec<&str> = match item.get("type").and_then(Value::as_str) {
							Some("text") => vec!["value", "color", "alignment", "font", "text-overflow"],
							Some("pixmap") => vec!["value"],
							Some("bar") => BAR_KEYS.to_vec(),
							Some("gbar") => BAR_KEYS.iter().copied().chain(["bar_h"]).collect(),
							Some(unknown) => {
								warn!("setFeedback: unknown item type '{unknown}' for key '{key}'");
								continue;
							}
							None => {
								warn!("setFeedback: item with key '{key}' has no type field");
								continue;
							}
						};

						let valid_keys: Vec<&str> = COMMON_KEYS.iter().copied().chain(type_keys).collect();
						let item_type = item.get("type").and_then(Value::as_str).unwrap_or("").to_string();

						for (field, field_value) in obj {
							if valid_keys.contains(&field.as_str()) {
								let coerced = if field == "value"
									&& matches!(item_type.as_str(), "bar" | "gbar")
									&& let Value::String(s) = field_value
									&& let Ok(n) = s.parse::<f64>()
									&& let Some(n) = serde_json::Number::from_f64(n)
								{
									Value::Number(n)
								} else {
									field_value.clone()
								};
								item[field] = coerced;
							} else {
								warn!("setFeedback: key '{key}' has unknown field '{field}' for its type, ignoring");
							}
						}
					}
					_ => {
						warn!("setFeedback: key '{key}' has unexpected payload type, ignoring");
					}
				}
			}

			if let Some(title_item) = items_array.iter_mut().find(|item| item.get("key").and_then(Value::as_str) == Some("title"))
				&& (title_item.get("value").is_none() || title_item["value"] == Value::Null)
			{
				let current_text = &action.states[action.current_state as usize].text;
				title_item["value"] = Value::String(current_text.clone());
			}

			if let Some(icon_item) = items_array.iter_mut().find(|item| item.get("key").and_then(Value::as_str) == Some("icon")) {
				let icon_empty = icon_item.get("value").and_then(Value::as_str).map_or(true, str::is_empty);

				debug!("setFeedback: icon_empty: {}", icon_empty);
				debug!("setFeedback: icon as str: {:?}", icon_item.get("value").and_then(Value::as_str));

				if icon_empty {
					let icon = action
						.states
						.get(action.current_state as usize)
						.map(|state| &state.image)
						.filter(|image| !image.is_empty())
						.unwrap_or(&action.action.icon);

					debug!("setFeedback: setting icon to: {}", icon);

					if !icon.is_empty() {
						icon_item["value"] = Value::String(icon.clone());
					}
				}
			}

			update_state(crate::APP_HANDLE.get().unwrap(), action.context.clone(), &mut locks).await?;
		}
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
