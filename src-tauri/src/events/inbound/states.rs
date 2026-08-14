use super::ContextAndPayloadEvent;

use crate::events::frontend::instances::update_state;
use crate::shared::ActionContext;
use crate::store::profiles::{LocksMut, acquire_locks_mut, get_instance_mut, mark_profile_stale};

use anyhow::bail;
use serde::Deserialize;
use serde_json::Value;

/// If the given context is a child of a Dial Stack or Action Wheel container, trigger an
/// update_state for the root container so the device LCD re-renders with the active child's display.
async fn maybe_update_dial_stack_parent(context: &ActionContext, locks: &mut LocksMut<'_>) -> Result<(), anyhow::Error> {
	if context.index == 0 {
		return Ok(());
	}
	let root_context = ActionContext {
		device: context.device.clone(),
		profile: context.profile.clone(),
		controller: context.controller.clone(),
		position: context.position,
		index: 0,
	};
	if let Some(root) = get_instance_mut(&root_context, locks).await?
		&& matches!(root.action.uuid.as_str(), "opendeck.dialstack" | "opendeck.actionwheel")
	{
		update_state(crate::APP_HANDLE.get().unwrap(), root_context, locks).await?;
	}
	Ok(())
}

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
	let _ = maybe_update_dial_stack_parent(&event.context, &mut locks).await;
	mark_profile_stale(&event.context.device, &mut locks).await?;

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

	let _ = maybe_update_dial_stack_parent(&event.context, &mut locks).await;
	mark_profile_stale(&event.context.device, &mut locks).await?;
	Ok(())
}

pub async fn set_feedback(event: ContextAndPayloadEvent<Value>) -> Result<(), anyhow::Error> {
	let mut locks = acquire_locks_mut().await;

	if let Some(instance) = get_instance_mut(&event.context, &mut locks).await?
		&& let Some(encoder) = &mut instance.action.encoder
	{
		let Some(layout) = &mut encoder.layout_parsed else {
			bail!("Layout is not loaded; cannot set feedback");
		};

		layout.set_feedback(event.payload)?;
		update_state(crate::APP_HANDLE.get().unwrap(), instance.context.clone(), &mut locks).await?;
	}

	let _ = maybe_update_dial_stack_parent(&event.context, &mut locks).await;
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
	let _ = maybe_update_dial_stack_parent(&event.context, &mut locks).await;
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
	let _ = maybe_update_dial_stack_parent(&event.context, &mut locks).await;
	mark_profile_stale(&event.context.device, &mut locks).await?;

	Ok(())
}
