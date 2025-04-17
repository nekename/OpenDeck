use std::str::FromStr;

use crate::events::outbound::settings as outbound;
use crate::shared::ActionContext;
use crate::store::profiles::{acquire_locks, acquire_locks_mut, get_instance, get_instance_mut, save_profile};

pub async fn set_settings(event: super::ContextAndPayloadEvent<serde_json::Value>, from_property_inspector: bool) -> Result<(), anyhow::Error> {
	let mut locks = acquire_locks_mut().await;

	if let Some(instance) = get_instance_mut(&event.context, &mut locks).await? {
		instance.settings = event.payload;
		outbound::did_receive_settings(instance, !from_property_inspector).await?;
		save_profile(&event.context.device, &mut locks).await?;
	}

	Ok(())
}

pub async fn get_settings(event: super::ContextEvent, from_property_inspector: bool) -> Result<(), anyhow::Error> {
	let locks = acquire_locks().await;

	if let Some(instance) = get_instance(&event.context, &locks).await? {
		outbound::did_receive_settings(instance, from_property_inspector).await?;
	}

	Ok(())
}

pub async fn set_global_settings(event: super::ContextAndPayloadEvent<serde_json::Value, String>, from_property_inspector: bool) -> Result<(), anyhow::Error> {
	let uuid = if from_property_inspector {
		if let Some(instance) = get_instance(&ActionContext::from_str(&event.context)?, &acquire_locks().await).await? {
			instance.action.plugin.clone()
		} else {
			return Ok(());
		}
	} else {
		event.context.clone()
	};

	{
		let settings_dir = crate::shared::config_dir().join("settings");
		tokio::fs::create_dir_all(&settings_dir).await?;

		let path = settings_dir.join(uuid.clone() + ".json");
		tokio::fs::write(path, event.payload.to_string()).await?;
	}

	outbound::did_receive_global_settings(&uuid, !from_property_inspector).await?;

	Ok(())
}

pub async fn get_global_settings(event: super::ContextEvent<String>, from_property_inspector: bool) -> Result<(), anyhow::Error> {
	let uuid = if from_property_inspector {
		if let Some(instance) = get_instance(&ActionContext::from_str(&event.context)?, &acquire_locks().await).await? {
			instance.action.plugin.clone()
		} else {
			return Ok(());
		}
	} else {
		event.context.clone()
	};

	outbound::did_receive_global_settings(&uuid, from_property_inspector).await
}
