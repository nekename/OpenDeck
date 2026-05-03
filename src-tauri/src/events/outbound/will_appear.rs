use super::{GenericInstancePayload, send_to_plugin};

use crate::shared::{ActionContext, ActionInstance};

use std::collections::HashSet;
use std::sync::LazyLock;
use tokio::sync::Mutex;

/// Encoder contexts that just received willAppear and haven't yet had their
/// first setFeedback processed. The fast path in set_feedback skips pushing
/// to the device for one cycle so the plugin's initial (possibly stale)
/// cached frame doesn't race with the profile switch's clear_screen.
static WARMING_UP: LazyLock<Mutex<HashSet<String>>> = LazyLock::new(|| Mutex::new(HashSet::new()));

pub async fn clear_warming_up(context: &str) -> bool {
	WARMING_UP.lock().await.remove(context)
}

#[derive(serde::Serialize)]
struct AppearEvent {
	event: &'static str,
	action: String,
	context: ActionContext,
	device: String,
	payload: GenericInstancePayload,
}

pub async fn will_appear(instance: &ActionInstance) -> Result<(), anyhow::Error> {
	if instance.context.controller == "Encoder" {
		WARMING_UP.lock().await.insert(instance.context.to_string());
	}
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
