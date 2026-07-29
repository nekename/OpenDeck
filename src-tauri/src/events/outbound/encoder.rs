use super::{Coordinates, send_to_plugin};

use crate::shared::ActionContext;
use crate::store::profiles::{acquire_locks_mut, get_instance_mut};

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
	let _ = crate::frontend::instances::key_moved(crate::APP_HANDLE.get().unwrap(), context.into(), event == "dialDown").await;

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
