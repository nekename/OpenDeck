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

/*
 * Extension spécifique à OpenDeck.
 *
 * Le protocole officiel Stream Deck ne définit pas d’événement
 * de swipe pour les plugins. OpenDeck transmet donc le geste à
 * l’action située sous le point de départ.
 */
#[derive(Serialize)]
#[allow(non_snake_case)]
struct TouchSwipePayload {
	controller: &'static str,
	settings: serde_json::Value,
	coordinates: Coordinates,
	startPos: (u16, u16),
	endPos: (u16, u16),
	delta: (i32, i32),
	direction: &'static str,
}

#[derive(Serialize)]
struct TouchSwipeEvent {
	event: &'static str,
	action: String,
	context: ActionContext,
	device: String,
	payload: TouchSwipePayload,
}

pub async fn touch_swipe(device: &str, index: u8, start_x: u16, start_y: u16, end_x: u16, end_y: u16) -> Result<(), anyhow::Error> {
	let delta_x = i32::from(end_x) - i32::from(start_x);
	let delta_y = i32::from(end_y) - i32::from(start_y);

	let direction = if delta_x.abs() >= delta_y.abs() {
		if delta_x < 0 { "left" } else { "right" }
	} else if delta_y < 0 {
		"up"
	} else {
		"down"
	};

	let mut locks = acquire_locks_mut().await;

	let selected_profile = locks.device_stores.get_selected_profile(device)?;

	let context = ActionContext {
		device: device.to_owned(),
		profile: selected_profile.to_owned(),
		controller: "Encoder".to_owned(),
		position: index,
		index: 0,
	};

	let Some(instance) = get_instance_mut(&context, &mut locks).await? else {
		log::debug!("Ignoring touch swipe: no encoder action at {}:{}", selected_profile, index);

		return Ok(());
	};

	log::debug!(
		"Touch swipe for {}: ({}, {}) -> ({}, {}), direction={}",
		instance.action.uuid,
		start_x,
		start_y,
		end_x,
		end_y,
		direction
	);

	send_to_plugin(
		&instance.action.plugin,
		&TouchSwipeEvent {
			event: "touchSwipe",
			action: instance.action.uuid.clone(),
			context: instance.context.clone(),
			device: instance.context.device.clone(),
			payload: TouchSwipePayload {
				controller: "Encoder",
				settings: instance.settings.clone(),
				coordinates: Coordinates { row: 0, column: index },
				startPos: (start_x, start_y),
				endPos: (end_x, end_y),
				delta: (delta_x, delta_y),
				direction,
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
	let Some(instance) = get_instance_mut(&context, &mut locks).await? else {
		return Ok(());
	};

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
