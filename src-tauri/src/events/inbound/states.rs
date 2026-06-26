use super::ContextAndPayloadEvent;

use crate::events::frontend::instances::update_state;
use crate::store::profiles::{acquire_locks_mut, debounce_profile_save, get_instance_mut, save_profile};

use anyhow::bail;
use log::warn;
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

	if let Some(instance) = get_instance_mut(&event.context, &mut locks).await?
		&& let Some(encoder) = &mut instance.action.encoder
		&& let Value::Object(map) = event.payload
	{
		let layout = &mut encoder.layout_parsed;
		if layout.is_null() {
			bail!("Layout is not loaded; cannot set feedback");
		}

		let Some(items_array) = layout.get_mut("items").and_then(Value::as_array_mut) else {
			bail!("Layout has no items array");
		};

		for (key, payload_value) in &map {
			// Grab the item from the layout
			let Some(item) = items_array.iter_mut().find(|item| {
				matches!(
					item.get("key").and_then(|v| v.as_str()),
					Some(k) if k == key
				)
			}) else {
				warn!("setFeedback: no layout item found for key '{key}'");
				continue;
			};

			match payload_value {
				// We have a direct value; find the key, and set it
				Value::String(_) | Value::Number(_) => {
					// Get the item type
					let Some(item_type) = item.get("type").and_then(Value::as_str) else {
						warn!("setFeedback: no type found for key '{key}'");
						continue;
					};

					match item_type {
						"text" => {
							// We need to map the value to a string
							if let Value::Number(number) = payload_value {
								item["value"] = Value::String(number.to_string());
							} else {
								// Clone the string
								item["value"] = payload_value.clone();
							}
						}

						"bar" | "gbar" => {
							if let Value::Number(value) = payload_value {
								if key == "value" {
									item["value"] = Value::Number(value.clone());
								}
							} else {
								warn!("setFeedback: bar/gbar expected number for key '{key}'");
							}
						}

						"pixmap" => {
							// Update the pixmap value; this should already be a string
							item["value"] = payload_value.clone();
						}

						// Ignore anything else
						_ => {
							warn!("setFeedback: unknown item type '{item_type}' for key '{key}'");
						}
					}
				}

				// We have an object, so we need to locate and map the change
				Value::Object(obj) => {
					// Get the item type
					let Some(item_type) = item.get("type").and_then(Value::as_str) else {
						warn!("setFeedback: missing or invalid 'type' field in item: {:?}", item);
						continue;
					};

					// Get the valid keys for this item type
					let type_keys: Vec<&str> = match item_type {
						"text" => vec!["value", "color", "alignment", "font", "text-overflow"],
						"pixmap" => vec!["value"],
						"bar" => BAR_KEYS.to_vec(),
						"gbar" => BAR_KEYS.iter().copied().chain(["bar_h"]).collect(),
						unknown => {
							warn!("setFeedback: unknown item type '{unknown}' for key '{key}'");
							continue;
						}
					};

					// Add the common keys
					let valid_keys: Vec<&str> = COMMON_KEYS.iter().copied().chain(type_keys).collect();

					// Iterate over the values in the object
					for (field, field_value) in obj {
						if valid_keys.contains(&field.as_str()) {
							item[field] = field_value.clone()
						}
					}
				}

				_ => {
					warn!("setFeedback: key '{key}' has unexpected payload type, ignoring");
				}
			}
		}

		update_state(crate::APP_HANDLE.get().unwrap(), instance.context.clone(), &mut locks).await?;
	}

	Ok(())
}

pub async fn set_feedback_layout(event: ContextAndPayloadEvent<SetFeedbackLayoutPayload>) -> Result<(), anyhow::Error> {
	let mut locks = acquire_locks_mut().await;
	if let Some(instance) = get_instance_mut(&event.context, &mut locks).await? {
		// We need to replace the existing parsed layout with the new one
		let layout_name = event.payload.layout.clone();
		crate::shared::initialise_encoder_layout(&mut instance.action, Some(layout_name))?;

		// Trigger a state update; should cause a redraw
		update_state(crate::APP_HANDLE.get().unwrap(), instance.context.clone(), &mut locks).await?;
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
