use super::{GenericInstancePayload, send_to_plugin};

use crate::shared::{ActionContext, ActionInstance, DEVICES};
use crate::store::profiles::acquire_locks_mut;

#[derive(serde::Serialize)]
struct AppearEvent {
	event: &'static str,
	action: String,
	context: ActionContext,
	device: String,
	payload: GenericInstancePayload,
}

pub async fn will_appear(instance: &ActionInstance) -> Result<(), anyhow::Error> {
	send_to_plugin(
		&instance.action.plugin,
		&AppearEvent {
			event: "willAppear",
			action: instance.action.uuid.clone(),
			context: instance.context.clone(),
			device: instance.context.device.clone(),
			payload: GenericInstancePayload::new(instance),
		},
	)
	.await?;

	super::states::title_parameters_did_change(instance, instance.current_state).await?;

	Ok(())
}

pub async fn will_disappear(instance: &ActionInstance, clear_on_device: bool) -> Result<(), anyhow::Error> {
	send_to_plugin(
		&instance.action.plugin,
		&AppearEvent {
			event: "willDisappear",
			action: instance.action.uuid.clone(),
			context: instance.context.clone(),
			device: instance.context.device.clone(),
			payload: GenericInstancePayload::new(instance),
		},
	)
	.await?;

	if clear_on_device && let Err(error) = crate::events::outbound::devices::update_image((&instance.context).into(), None).await {
		log::warn!("Failed to clear device image: {}", error);
	}

	Ok(())
}

/// Refresh the current profile for a device by re-sending willAppear events for all instances.
/// This is used to restore the device state after unlocking the screen.
pub async fn refresh_profile(device_id: &str) -> Result<(), anyhow::Error> {
	let mut locks = acquire_locks_mut().await;
	let Some(device_info) = DEVICES.get(device_id) else {
		return Ok(());
	};

	let selected_profile = locks.device_stores.get_selected_profile(device_id)?;
	let store = locks.profile_stores.get_profile_store(&device_info, &selected_profile)?;
	let profile = &store.value;

	for instance in profile.keys.iter().flatten().chain(&mut profile.sliders.iter().flatten()) {
		if !matches!(instance.action.uuid.as_str(), "opendeck.multiaction" | "opendeck.toggleaction") {
			let _ = will_appear(instance).await;
		} else {
			for child in instance.children.as_ref().unwrap() {
				let _ = will_appear(child).await;
			}
		}
	}

	Ok(())
}
