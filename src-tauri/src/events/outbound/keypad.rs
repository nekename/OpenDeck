use super::{GenericInstancePayload, send_to_plugin};

use crate::events::frontend::instances::{key_moved, update_state};
use crate::shared::{ActionContext, ActionInstance, Context};
use crate::store::profiles::{acquire_locks_mut, get_instance_mut, get_slot_mut, mark_profile_stale};

use std::sync::LazyLock;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use dashmap::DashMap;
use serde::Serialize;

static KEY_DOWN_TARGETS: LazyLock<DashMap<(String, u8), Context>> = LazyLock::new(DashMap::new);

/// Keys awaiting a potential second click, mapped to the generation of their timeout task.
static DOUBLE_CLICK_PENDING: LazyLock<DashMap<(String, u8), u64>> = LazyLock::new(DashMap::new);
static DOUBLE_CLICK_GENERATION: AtomicU64 = AtomicU64::new(0);

const DEFAULT_DOUBLE_CLICK_WINDOW: u64 = 400;

#[derive(Serialize)]
struct KeyEvent {
	event: &'static str,
	action: String,
	context: ActionContext,
	device: String,
	payload: GenericInstancePayload,
}

/// Sends a full key press (down + up) to a child of a Double Click action and updates its state.
async fn press_double_click_child(child: ActionInstance) -> Result<(), anyhow::Error> {
	send_to_plugin(
		&child.action.plugin,
		&KeyEvent {
			event: "keyDown",
			action: child.action.uuid.clone(),
			context: child.context.clone(),
			device: child.context.device.clone(),
			payload: GenericInstancePayload::new(&child),
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
			payload: GenericInstancePayload::new(&child),
		},
	)
	.await?;

	let mut locks = acquire_locks_mut().await;
	if let Some(instance) = get_instance_mut(&child.context, &mut locks).await? {
		if instance.states.len() == 2 && !instance.action.disable_automatic_states {
			instance.current_state = (instance.current_state + 1) % (instance.states.len() as u16);
		}
		let context = instance.context.clone();
		let _ = update_state(crate::APP_HANDLE.get().unwrap(), context, &mut locks).await;
	}
	mark_profile_stale(&child.context.device, &mut locks).await?;

	Ok(())
}

pub async fn key_down(device: &str, key: u8) -> Result<(), anyhow::Error> {
	let mut locks = acquire_locks_mut().await;
	let selected_profile = locks.device_stores.get_selected_profile(device)?;
	let context = Context {
		device: device.to_owned(),
		profile: selected_profile.to_owned(),
		controller: "Keypad".to_owned(),
		position: key,
	};

	let _ = key_moved(crate::APP_HANDLE.get().unwrap(), context.clone(), true).await;
	KEY_DOWN_TARGETS.insert((device.to_owned(), key), context.clone());

	let Some(instance) = get_slot_mut(&context, &mut locks).await? else { return Ok(()) };
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
	} else if instance.action.uuid == "opendeck.doubleclickaction" {
		// Single versus double click is resolved on key up.
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
	let selected_profile = locks.device_stores.get_selected_profile(device)?;
	let context = Context {
		device: device.to_owned(),
		profile: selected_profile.to_owned(),
		controller: "Keypad".to_owned(),
		position: key,
	};

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
	} else if instance.action.uuid == "opendeck.doubleclickaction" {
		let children = instance.children.clone().unwrap_or_default();
		let window = instance.settings.get("double_click_window").and_then(|v| v.as_u64()).unwrap_or(DEFAULT_DOUBLE_CLICK_WINDOW);
		let map_key = (device.to_owned(), key);

		if DOUBLE_CLICK_PENDING.remove(&map_key).is_some() {
			// Second click within the window: trigger the double click child.
			if let Some(child) = children.get(1).cloned() {
				tokio::spawn(async move {
					if let Err(error) = press_double_click_child(child).await {
						log::warn!("Failed to trigger double click action: {}", error);
					}
				});
			}
		} else {
			let generation = DOUBLE_CLICK_GENERATION.fetch_add(1, Ordering::Relaxed);
			DOUBLE_CLICK_PENDING.insert(map_key.clone(), generation);
			let child = children.first().cloned();
			tokio::spawn(async move {
				tokio::time::sleep(Duration::from_millis(window)).await;
				// If the entry is gone or was replaced, a second click arrived in the meantime.
				if DOUBLE_CLICK_PENDING.remove_if(&map_key, |_, v| *v == generation).is_none() {
					return;
				}
				let Some(child) = child else { return };
				if let Err(error) = press_double_click_child(child).await {
					log::warn!("Failed to trigger single click action: {}", error);
				}
			});
		}
	} else if instance.action.uuid != "opendeck.multiaction" {
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
