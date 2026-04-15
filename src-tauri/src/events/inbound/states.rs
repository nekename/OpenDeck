use super::ContextAndPayloadEvent;

use crate::events::frontend::instances::update_state;
use crate::store::profiles::{acquire_locks_mut, debounce_profile_save, get_instance_mut, save_profile};

use serde::Deserialize;

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

/// Merge a setFeedback payload into the instance's persistent feedback state
/// and notify the frontend to re-render the layout. Keys not present in the
/// current layout are still stored (they have no visual effect, but the spec
/// says unrecognised keys are ignored rather than an error).
pub async fn set_feedback(event: ContextAndPayloadEvent<serde_json::Value>) -> Result<(), anyhow::Error> {
	let mut locks = acquire_locks_mut().await;
	if let Some(instance) = get_instance_mut(&event.context, &mut locks).await? {
		merge_feedback(&mut instance.feedback, event.payload);
		let snapshot = instance.clone();
		drop(locks);
		emit_feedback_changed(&snapshot);
	}
	Ok(())
}

/// Handle setFeedbackLayout: record the new layout, reset accumulated feedback
/// state (spec-level keys differ per layout so stale state would be wrong),
/// and notify the frontend.
pub async fn set_feedback_layout(event: ContextAndPayloadEvent<serde_json::Value>) -> Result<(), anyhow::Error> {
	let layout_id = match &event.payload {
		serde_json::Value::String(s) => s.clone(),
		serde_json::Value::Object(obj) => obj.get("layout").and_then(|v| v.as_str()).map(str::to_owned).unwrap_or_default(),
		_ => return Ok(()),
	};
	if layout_id.is_empty() {
		return Ok(());
	}
	let mut locks = acquire_locks_mut().await;
	if let Some(instance) = get_instance_mut(&event.context, &mut locks).await? {
		instance.feedback_layout = Some(layout_id);
		instance.feedback = serde_json::Value::Null;
		let snapshot = instance.clone();
		drop(locks);
		emit_feedback_changed(&snapshot);
	}
	Ok(())
}

fn merge_feedback(target: &mut serde_json::Value, patch: serde_json::Value) {
	if target.is_null() {
		*target = serde_json::Value::Object(serde_json::Map::new());
	}
	let serde_json::Value::Object(target_obj) = target else { return };
	let serde_json::Value::Object(patch_obj) = patch else { return };
	for (key, value) in patch_obj {
		target_obj.insert(key, value);
	}
}

fn emit_feedback_changed(instance: &crate::shared::ActionInstance) {
	use tauri::{Emitter, Manager};
	let Some(app) = crate::APP_HANDLE.get() else { return };
	let Some(window) = app.get_webview_window("main") else { return };
	let _ = window.emit(
		"feedback_changed",
		serde_json::json!({
			"context": instance.context.to_string(),
			"plugin": instance.action.plugin,
			"layout": instance.feedback_layout,
			"feedback": instance.feedback,
		}),
	);
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
