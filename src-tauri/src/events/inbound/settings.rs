use crate::events::outbound::settings as outbound;
use crate::shared::ActionContext;
use crate::store::profiles::{acquire_locks, acquire_locks_mut, debounce_profile_save, get_instance, get_instance_mut};

use std::io::Write;
use std::str::FromStr;

pub async fn set_settings(event: super::ContextAndPayloadEvent<serde_json::Value>, from_property_inspector: bool) -> Result<(), anyhow::Error> {
	let mut locks = acquire_locks_mut().await;

	if let Some(instance) = get_instance_mut(&event.context, &mut locks).await? {
		if instance.settings == event.payload {
			outbound::did_receive_settings(instance, !from_property_inspector).await?;
			return Ok(());
		}
		instance.settings = event.payload;
		outbound::did_receive_settings(instance, !from_property_inspector).await?;
		debounce_profile_save(event.context);
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
		let contents = event.payload.to_string();

		if !std::fs::read_to_string(&path).map(|existing| existing == contents).unwrap_or(false) {
			let mut file = std::fs::OpenOptions::new().write(true).truncate(true).create(true).open(path)?;
			file.lock()?;
			file.write_all(contents.as_bytes())?;
			file.sync_data()?;
			file.unlock()?;
		}
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
