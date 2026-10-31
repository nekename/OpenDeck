use super::{GenericInstancePayload, send_to_plugin};

use crate::events::frontend::instances::{key_moved, update_state};
use crate::shared::{ActionContext, ActionInstance, Context, is_page_action};
use crate::store::profiles::{acquire_locks_mut, get_slot_mut, mark_profile_stale, selected_slot_context};

use std::sync::LazyLock;
use std::time::Duration;

use dashmap::DashMap;
use serde::Serialize;

static KEY_DOWN_TARGETS: LazyLock<DashMap<(String, u8), Context>> = LazyLock::new(DashMap::new);

#[derive(Serialize)]
struct KeyEvent {
	event: &'static str,
	action: String,
	context: ActionContext,
	device: String,
	payload: GenericInstancePayload,
}

async fn activate_page_action(device: &str, instance: &ActionInstance) -> Result<(), anyhow::Error> {
	match instance.action.uuid.as_str() {
		"opendeck.nextpage" => {
			crate::events::frontend::profiles::next_page(device).await?;
		}
		"opendeck.prevpage" => {
			crate::events::frontend::profiles::previous_page(device).await?;
		}
		"opendeck.gotopage" => {
			let page = instance.settings.get("page").and_then(|value| value.as_u64()).unwrap_or(1) as usize;
			crate::events::frontend::profiles::goto_page(device, page).await?;
		}
		_ => {}
	}
	Ok(())
}

pub async fn key_down(device: &str, key: u8) -> Result<(), anyhow::Error> {
	let mut locks = acquire_locks_mut().await;
	let context = selected_slot_context(device, "Keypad", key, &mut locks)?;

	let _ = key_moved(crate::APP_HANDLE.get().unwrap(), context.clone(), true).await;
	KEY_DOWN_TARGETS.insert((device.to_owned(), key), context.clone());

	let Some(instance) = get_slot_mut(&context, &mut locks).await? else { return Ok(()) };
	if is_page_action(&instance.action.uuid) {
		let instance = instance.clone();
		drop(locks);
		return activate_page_action(device, &instance).await;
	}
	if instance.action.uuid == "opendeck.multiaction" {
		let children = instance.children.clone().unwrap_or_default();
		let delays: Vec<u64> = instance
			.settings
			.get("delays")
			.and_then(|v| v.as_array())
			.map(|arr| arr.iter().filter_map(|v| v.as_u64()).collect())
			.unwrap_or_default();

		drop(locks);

		for (i, child) in children.iter().enumerate() {
			if is_page_action(&child.action.uuid) {
				activate_page_action(device, child).await?;
				let delay = delays.get(i).copied().unwrap_or(100);
				if delay > 0 {
					tokio::time::sleep(Duration::from_millis(delay)).await;
				}
				continue;
			}
			send_to_plugin(
				&child.action.plugin,
				&KeyEvent {
					event: "keyDown",
					action: child.action.uuid.clone(),
					context: child.context.clone(),
					device: child.context.device.clone(),
					payload: GenericInstancePayload::new(child),
				},
			)
			.await?;

			tokio::time::sleep(Duration::from_millis(100)).await;

			send_to_plugin(
				&child.action.plugin,
				&KeyEvent {
					event: "keyUp",
					action: child.action.uuid.clone(),
					context: child.context.clone(),
					device: child.context.device.clone(),
					payload: GenericInstancePayload::new(child),
				},
			)
			.await?;

			let delay = delays.get(i).copied().unwrap_or(100);
			if delay > 0 {
				tokio::time::sleep(Duration::from_millis(delay)).await;
			}
		}

		let mut locks = acquire_locks_mut().await;

		if let Some(instance) = get_slot_mut(&context, &mut locks).await?
			&& let Some(children) = &mut instance.children
		{
			for child in &mut *children {
				if child.states.len() == 2 && !child.action.disable_automatic_states {
					child.current_state = (child.current_state + 1) % (child.states.len() as u16);
				}
			}

			for child in children.iter().map(|x| x.context.clone()).collect::<Vec<_>>() {
				let _ = update_state(crate::APP_HANDLE.get().unwrap(), child, &mut locks).await;
			}
		}

		mark_profile_stale(device, &mut locks).await?;
	} else if instance.action.uuid == "opendeck.toggleaction" {
		let children = instance.children.as_ref().unwrap();
		if children.is_empty() {
			return Ok(());
		}
		let child = &children[instance.current_state as usize];
		send_to_plugin(
			&child.action.plugin,
			&KeyEvent {
				event: "keyDown",
				action: child.action.uuid.clone(),
				context: child.context.clone(),
				device: child.context.device.clone(),
				payload: GenericInstancePayload::new(child),
			},
		)
		.await?;
	} else {
		send_to_plugin(
			&instance.action.plugin,
			&KeyEvent {
				event: "keyDown",
				action: instance.action.uuid.clone(),
				context: instance.context.clone(),
				device: instance.context.device.clone(),
				payload: GenericInstancePayload::new(instance),
			},
		)
		.await?;
	}

	Ok(())
}

pub async fn key_up(device: &str, key: u8) -> Result<(), anyhow::Error> {
	let mut locks = acquire_locks_mut().await;
	let context = selected_slot_context(device, "Keypad", key, &mut locks)?;

	let _ = key_moved(crate::APP_HANDLE.get().unwrap(), context.clone(), false).await;
	let Some((_, expected_context)) = KEY_DOWN_TARGETS.remove(&(device.to_owned(), key)) else {
		return Ok(());
	};
	if context != expected_context {
		return Ok(());
	}

	let slot = get_slot_mut(&context, &mut locks).await?;
	let Some(instance) = slot else { return Ok(()) };

	if instance.action.uuid == "opendeck.toggleaction" {
		let index = instance.current_state as usize;
		let children = instance.children.as_ref().unwrap();
		if children.is_empty() {
			return Ok(());
		}
		let child = &children[index];
		send_to_plugin(
			&child.action.plugin,
			&KeyEvent {
				event: "keyUp",
				action: child.action.uuid.clone(),
				context: child.context.clone(),
				device: child.context.device.clone(),
				payload: GenericInstancePayload::new(child),
			},
		)
		.await?;
		instance.current_state = ((index + 1) % instance.children.as_ref().unwrap().len()) as u16;
	} else if instance.action.uuid != "opendeck.multiaction" && !is_page_action(&instance.action.uuid) {
		if instance.states.len() == 2 && !instance.action.disable_automatic_states {
			instance.current_state = (instance.current_state + 1) % (instance.states.len() as u16);
		}
		send_to_plugin(
			&instance.action.plugin,
			&KeyEvent {
				event: "keyUp",
				action: instance.action.uuid.clone(),
				context: instance.context.clone(),
				device: instance.context.device.clone(),
				payload: GenericInstancePayload::new(instance),
			},
		)
		.await?;
	};

	let _ = update_state(crate::APP_HANDLE.get().unwrap(), instance.context.clone(), &mut locks).await;
	mark_profile_stale(device, &mut locks).await?;

	Ok(())
}
