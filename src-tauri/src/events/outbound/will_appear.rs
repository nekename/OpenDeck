use super::{GenericInstancePayload, send_to_plugin};

use crate::shared::{ActionContext, ActionInstance, Profile};

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

async fn for_visible_instances(instance: &ActionInstance, appear: bool, clear_on_device: bool) {
	if !matches!(instance.action.uuid.as_str(), "opendeck.multiaction" | "opendeck.toggleaction") {
		let _ = if appear { will_appear(instance).await } else { will_disappear(instance, clear_on_device).await };
	} else if let Some(children) = instance.children.as_ref() {
		for child in children {
			let _ = if appear { will_appear(child).await } else { will_disappear(child, clear_on_device).await };
		}
	}
}

pub async fn appear_page(profile: &Profile) {
	for instance in profile.current().slots().flatten() {
		for_visible_instances(instance, true, false).await;
	}
}

pub async fn disappear_page(profile: &Profile, clear_on_device: bool) {
	for instance in profile.current().slots().flatten() {
		for_visible_instances(instance, false, clear_on_device).await;
	}
}
