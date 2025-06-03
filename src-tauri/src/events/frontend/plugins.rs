use crate::shared::{config_dir, log_dir};

use super::Error;

use tauri::{AppHandle, Manager, command};
use tokio::fs;

#[derive(serde::Serialize)]
pub struct PluginInfo {
	id: String,
	name: String,
	author: String,
	icon: String,
	version: String,
	builtin: bool,
	registered: bool,
}

#[command]
pub async fn list_plugins(app: AppHandle) -> Result<Vec<PluginInfo>, Error> {
	let mut plugins = vec![];

	let mut entries = match fs::read_dir(&config_dir().join("plugins")).await {
		Ok(entries) => entries,
		Err(error) => return Err(anyhow::Error::from(error).into()),
	};

	let registered = crate::events::registered_plugins().await;
	let builtins = match app.path().resolve("plugins", tauri::path::BaseDirectory::Resource).map(std::fs::read_dir) {
		Ok(Ok(entries)) => entries.flatten().map(|x| x.file_name().to_str().unwrap().to_owned()).collect(),
		_ => vec![],
	};

	while let Ok(Some(entry)) = entries.next_entry().await {
		let path = match entry.metadata().await.unwrap().is_symlink() {
			true => fs::read_link(entry.path()).await.unwrap(),
			false => entry.path(),
		};
		let metadata = fs::metadata(&path).await.unwrap();
		if metadata.is_dir() {
			let id = path.file_name().unwrap().to_str().unwrap().to_owned();
			let Ok(manifest) = crate::plugins::manifest::read_manifest(&path) else {
				continue;
			};
			plugins.push(PluginInfo {
				name: manifest.name,
				author: manifest.author,
				icon: crate::shared::convert_icon(path.join(manifest.icon).to_str().unwrap().to_owned()),
				version: manifest.version,
				builtin: builtins.contains(&id),
				registered: registered.contains(&id),
				id,
			});
		}
	}

	Ok(plugins)
}

#[command]
pub async fn install_plugin(app: AppHandle, url: Option<String>, file: Option<String>, fallback_id: Option<String>) -> Result<(), Error> {
	let bytes = match file {
		None => {
			let resp = match reqwest::get(url.unwrap()).await {
				Ok(resp) => resp,
				Err(error) => return Err(anyhow::Error::from(error).into()),
			};
			use std::ops::Deref;
			match resp.bytes().await {
				Ok(bytes) => bytes.deref().to_owned(),
				Err(error) => return Err(anyhow::Error::from(error).into()),
			}
		}
		Some(path) => match std::fs::read(path) {
			Ok(bytes) => bytes,
			Err(error) => return Err(anyhow::Error::from(error).into()),
		},
	};

	let id = match crate::zip_extract::dir_name(std::io::Cursor::new(&bytes)) {
		Ok(id) => {
			log::trace!("Found directory with name {id} within archive");
			id
		}
		Err(error) => match fallback_id {
			Some(id) => format!("{id}.sdPlugin"),
			None => return Err(anyhow::Error::from(error).into()),
		},
	};

	let _ = crate::plugins::deactivate_plugin(&app, &id).await;

	let config_dir = config_dir();
	let actual = config_dir.join("plugins").join(&id);

	if actual.exists() {
		let _ = fs::create_dir_all(config_dir.join("temp")).await;
	}
	let temp = config_dir.join("temp").join(&id);
	let _ = fs::rename(&actual, &temp).await;

	if let Err(error) = crate::zip_extract::extract(std::io::Cursor::new(bytes), &config_dir.join("plugins")) {
		log::error!("Failed to unzip file: {}", error);
		let _ = fs::rename(&temp, &actual).await;
		let _ = crate::plugins::initialise_plugin(&actual).await;
		return Err(anyhow::Error::from(error).into());
	}
	if let Err(error) = crate::plugins::initialise_plugin(&actual).await {
		log::warn!("Failed to initialise plugin at {}: {}", actual.display(), error);
		let _ = fs::remove_dir_all(&actual).await;
		let _ = fs::rename(&temp, &actual).await;
		let _ = crate::plugins::initialise_plugin(&actual).await;
		return Err(error.into());
	}
	let _ = fs::remove_dir_all(config_dir.join("temp")).await;

	use tauri_plugin_aptabase::EventTracker;
	let _ = app.track_event("plugin_installed", Some(serde_json::json!({ "id": id.strip_suffix(".sdPlugin").unwrap_or(&id) })));

	Ok(())
}

#[command]
pub async fn remove_plugin(app: AppHandle, id: String) -> Result<(), Error> {
	let locks = crate::store::profiles::acquire_locks().await;
	let all = locks.profile_stores.all_from_plugin(&id);
	drop(locks);

	for context in all {
		super::instances::remove_instance(context).await?;
	}

	crate::plugins::deactivate_plugin(&app, &id).await?;
	if let Err(error) = fs::remove_dir_all(config_dir().join("plugins").join(&id)).await {
		return Err(anyhow::Error::from(error).into());
	}

	let mut categories = crate::shared::CATEGORIES.write().await;
	for category in categories.values_mut() {
		category.retain(|v| v.plugin != id);
	}
	categories.retain(|_, v| !v.is_empty());

	let _ = fs::remove_file(log_dir().join("plugins").join(format!("{id}.log"))).await;
	let _ = fs::remove_file(config_dir().join("settings").join(format!("{id}.json"))).await;

	Ok(())
}

#[command]
pub async fn reload_plugin(app: AppHandle, id: String) {
	let _ = crate::plugins::deactivate_plugin(&app, &id).await;
	let _ = crate::plugins::initialise_plugin(&config_dir().join("plugins").join(id)).await;
}
