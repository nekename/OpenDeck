use super::{Coordinates, GenericInstancePayload, send_to_plugin};

use crate::events::frontend::instances::{key_moved, update_state};
use crate::events::outbound::will_appear::{will_appear, will_appear_with_controller, will_disappear, will_disappear_with_controller};
use crate::shared::ActionContext;
use crate::store::profiles::{acquire_locks_mut, get_instance_mut, save_profile_now};

use serde::Serialize;

#[derive(Serialize)]
struct DialRotatePayload {
	controller: &'static str,
	settings: serde_json::Value,
	coordinates: Coordinates,
	ticks: i16,
	pressed: bool,
}

#[derive(Serialize)]
struct DialRotateEvent {
	event: &'static str,
	action: String,
	context: ActionContext,
	device: String,
	payload: DialRotatePayload,
}

pub async fn dial_rotate(device: &str, index: u8, ticks: i16) -> Result<(), anyhow::Error> {
	let mut locks = acquire_locks_mut().await;
	let selected_profile = locks.device_stores.get_selected_profile(device)?;
	let context = ActionContext {
		device: device.to_owned(),
		profile: selected_profile.to_owned(),
		controller: "Encoder".to_owned(),
		position: index,
		index: 0,
	};
	let Some(instance) = get_instance_mut(&context, &mut locks).await? else { return Ok(()) };

	if instance.action.uuid == "opendeck.dialstack" {
		let children = instance.children.clone().unwrap_or_default();
		if children.is_empty() {
			return Ok(());
		}
		let child = children[instance.current_state as usize].clone();
		drop(locks);
		send_to_plugin(
			&child.action.plugin,
			&DialRotateEvent {
				event: "dialRotate",
				action: child.action.uuid.clone(),
				context: child.context.clone(),
				device: child.context.device.clone(),
				payload: DialRotatePayload {
					controller: "Encoder",
					settings: child.settings.clone(),
					coordinates: Coordinates { row: 0, column: index },
					ticks,
					pressed: false,
				},
			},
		)
		.await
	} else if instance.action.uuid == "opendeck.actionwheel" {
		let children = instance.children.clone().unwrap_or_default();
		if children.is_empty() {
			return Ok(());
		}
		let len = children.len() as isize;
		let new_index = (instance.current_state as isize + ticks as isize).rem_euclid(len) as usize;
		let old_index = instance.current_state as usize;
		instance.current_state = new_index as u16;
		let old_child = children[old_index].clone();
		let new_child = children[new_index].clone();
		let _ = update_state(crate::APP_HANDLE.get().unwrap(), context.clone(), &mut locks).await;
		save_profile_now(device, &mut locks).await?;
		drop(locks);

		let _ = will_disappear_with_controller(&old_child, false, Some("Keypad")).await;
		let _ = will_appear_with_controller(&new_child, Some("Keypad")).await;
		Ok(())
	} else {
		send_to_plugin(
			&instance.action.plugin,
			&DialRotateEvent {
				event: "dialRotate",
				action: instance.action.uuid.clone(),
				context: instance.context.clone(),
				device: instance.context.device.clone(),
				payload: DialRotatePayload {
					controller: "Encoder",
					settings: instance.settings.clone(),
					coordinates: Coordinates { row: 0, column: index },
					ticks,
					pressed: false,
				},
			},
		)
		.await
	}
}

#[derive(Serialize)]
struct DialPressPayload {
	controller: &'static str,
	settings: serde_json::Value,
	coordinates: Coordinates,
}

#[derive(Serialize)]
struct DialPressEvent {
	event: &'static str,
	action: String,
	context: ActionContext,
	device: String,
	payload: DialPressPayload,
}

#[derive(Serialize)]
struct KeyEvent {
	event: &'static str,
	action: String,
	context: ActionContext,
	device: String,
	payload: GenericInstancePayload,
}

pub async fn dial_press(device: &str, event: &'static str, index: u8) -> Result<(), anyhow::Error> {
	let mut locks = acquire_locks_mut().await;
	let selected_profile = locks.device_stores.get_selected_profile(device)?;
	let context = ActionContext {
		device: device.to_owned(),
		profile: selected_profile.to_owned(),
		controller: "Encoder".to_owned(),
		position: index,
		index: 0,
	};
	let Some(instance) = get_instance_mut(&context, &mut locks).await? else { return Ok(()) };

	if instance.action.uuid == "opendeck.dialstack" {
		let _ = key_moved(crate::APP_HANDLE.get().unwrap(), context.clone().into(), event == "dialDown").await;

		if event != "dialDown" {
			return Ok(());
		}

		let children = instance.children.clone().unwrap_or_default();
		if children.is_empty() {
			return Ok(());
		}
		let old_index = instance.current_state as usize;
		let new_index = (old_index + 1) % children.len();
		let old_child = children[old_index].clone();
		let new_child = children[new_index].clone();
		instance.current_state = new_index as u16;

		let _ = update_state(crate::APP_HANDLE.get().unwrap(), context.clone(), &mut locks).await;
		save_profile_now(device, &mut locks).await?;
		drop(locks);

		let _ = will_disappear(&old_child, false).await;
		let _ = will_appear(&new_child).await;

		return Ok(());
	}

	if instance.action.uuid == "opendeck.actionwheel" {
		let _ = key_moved(crate::APP_HANDLE.get().unwrap(), context.clone().into(), event == "dialDown").await;

		let children = instance.children.clone().unwrap_or_default();
		if children.is_empty() {
			return Ok(());
		}
		let child = children[instance.current_state as usize].clone();
		drop(locks);

		// Translate dialDown/dialUp to keyDown/keyUp for the key-style child action.
		let key_event = match event {
			"dialDown" => "keyDown",
			"dialUp" => "keyUp",
			other => other,
		};

		send_to_plugin(
			&child.action.plugin,
			&KeyEvent {
				event: key_event,
				action: child.action.uuid.clone(),
				context: child.context.clone(),
				device: child.context.device.clone(),
				payload: GenericInstancePayload::new_with_controller(&child, "Keypad"),
			},
		)
		.await
	} else {
		let _ = key_moved(crate::APP_HANDLE.get().unwrap(), context.into(), event == "dialDown").await;

		send_to_plugin(
			&instance.action.plugin,
			&DialPressEvent {
				event,
				action: instance.action.uuid.clone(),
				context: instance.context.clone(),
				device: instance.context.device.clone(),
				payload: DialPressPayload {
					controller: "Encoder",
					settings: instance.settings.clone(),
					coordinates: Coordinates { row: 0, column: index },
				},
			},
		)
		.await
	}
}

#[derive(Serialize)]
#[allow(non_snake_case)]
struct TouchTapPayload {
	controller: &'static str,
	settings: serde_json::Value,
	coordinates: Coordinates,
	tapPos: (u16, u16),
	hold: bool,
}

#[derive(Serialize)]
struct TouchTapEvent {
	event: &'static str,
	action: String,
	context: ActionContext,
	device: String,
	payload: TouchTapPayload,
}

pub async fn touch_tap(device: &str, index: u8, x: u16, y: u16, hold: bool) -> Result<(), anyhow::Error> {
	let mut locks = acquire_locks_mut().await;
	let selected_profile = locks.device_stores.get_selected_profile(device)?;
	let context = ActionContext {
		device: device.to_owned(),
		profile: selected_profile.to_owned(),
		controller: "Encoder".to_owned(),
		position: index,
		index: 0,
	};
	let Some(instance) = get_instance_mut(&context, &mut locks).await? else { return Ok(()) };

	if instance.action.uuid == "opendeck.dialstack" {
		let children = instance.children.clone().unwrap_or_default();
		if children.is_empty() {
			return Ok(());
		}
		let child = children[instance.current_state as usize].clone();
		drop(locks);
		send_to_plugin(
			&child.action.plugin,
			&TouchTapEvent {
				event: "touchTap",
				action: child.action.uuid.clone(),
				context: child.context.clone(),
				device: child.context.device.clone(),
				payload: TouchTapPayload {
					controller: "Encoder",
					settings: child.settings.clone(),
					coordinates: Coordinates { row: 0, column: index },
					tapPos: (x, y),
					hold,
				},
			},
		)
		.await
	} else if instance.action.uuid == "opendeck.actionwheel" {
		// Action Wheel holds key-style actions which don't define touch behavior; consume.
		Ok(())
	} else {
		send_to_plugin(
			&instance.action.plugin,
			&TouchTapEvent {
				event: "touchTap",
				action: instance.action.uuid.clone(),
				context: instance.context.clone(),
				device: instance.context.device.clone(),
				payload: TouchTapPayload {
					controller: "Encoder",
					settings: instance.settings.clone(),
					coordinates: Coordinates { row: 0, column: index },
					tapPos: (x, y),
					hold,
				},
			},
		)
		.await
	}
}
