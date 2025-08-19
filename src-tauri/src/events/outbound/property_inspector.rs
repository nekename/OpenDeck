use crate::shared::ActionContext;
use crate::store::profiles::acquire_locks;

use serde::Serialize;

#[derive(Serialize)]
struct SendToEvent {
	event: &'static str,
	action: String,
	context: ActionContext,
	payload: serde_json::Value,
}

pub async fn send_to_plugin(context: ActionContext, message: serde_json::Value) -> Result<(), anyhow::Error> {
	if let Some(instance) = crate::store::profiles::get_instance(&context, &acquire_locks().await).await? {
		super::send_to_plugin(
			&instance.action.plugin,
			&SendToEvent {
				event: "sendToPlugin",
				action: instance.action.uuid.clone(),
				context,
				payload: message,
			},
		)
		.await?;
	}

	Ok(())
}

pub async fn send_to_property_inspector(context: ActionContext, message: serde_json::Value) -> Result<(), anyhow::Error> {
	if let Some(instance) = crate::store::profiles::get_instance(&context, &acquire_locks().await).await? {
		super::send_to_property_inspector(
			&context,
			&SendToEvent {
				event: "sendToPropertyInspector",
				action: instance.action.uuid.clone(),
				context: context.clone(),
				payload: message,
			},
		)
		.await?;
	}

	Ok(())
}

#[derive(Serialize)]
struct PropertyInspectorDidAppearEvent {
	event: &'static str,
	action: String,
	context: ActionContext,
	device: String,
}

pub async fn property_inspector_did_appear(context: ActionContext, event: &'static str) -> Result<(), anyhow::Error> {
	if let Some(instance) = crate::store::profiles::get_instance(&context, &acquire_locks().await).await? {
		if instance.action.property_inspector.is_empty() {
			return Ok(());
		}
		super::send_to_plugin(
			&instance.action.plugin,
			&PropertyInspectorDidAppearEvent {
				event,
				action: instance.action.uuid.clone(),
				device: context.device.clone(),
				context,
			},
		)
		.await?;
	}

	Ok(())
}
