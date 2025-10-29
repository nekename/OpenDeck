use crate::store::{NotProfile, Store};

use std::collections::HashMap;

use active_win_pos_rs::get_active_window;
use once_cell::sync::Lazy;
use sysinfo::{Pid, ProcessRefreshKind, ProcessesToUpdate, RefreshKind, System};
use tauri::{Emitter, Manager};
use tokio::sync::RwLock;

pub type ApplicationProfiles = HashMap<String, HashMap<String, String>>;
impl NotProfile for ApplicationProfiles {}

pub static APPLICATIONS: RwLock<Vec<String>> = RwLock::const_new(Vec::new());
pub static APPLICATION_PROFILES: Lazy<RwLock<Store<ApplicationProfiles>>> = Lazy::new(|| RwLock::new(Store::new("applications", &crate::shared::config_dir(), HashMap::new()).unwrap()));

pub static APPLICATION_PROCESSES: Lazy<RwLock<HashMap<String, Vec<u32>>>> = Lazy::new(|| RwLock::new(HashMap::new()));
pub static APPLICATION_PLUGINS: Lazy<RwLock<HashMap<String, Vec<String>>>> = Lazy::new(|| RwLock::new(HashMap::new()));

#[derive(Clone, serde::Serialize)]
pub struct SwitchProfileEvent {
	device: String,
	profile: String,
}

pub fn init_application_watcher() {
	tokio::spawn(async move {
		let mut previous = String::new();
		let app_handle = crate::APP_HANDLE.get().unwrap();
		loop {
			let app_name = if let Ok(win) = get_active_window() {
				let mut applications = APPLICATIONS.write().await;
				if !applications.contains(&win.app_name) && !win.app_name.to_lowercase().starts_with(&crate::shared::PRODUCT_NAME.to_lowercase()) && !win.app_name.trim().is_empty() {
					applications.push(win.app_name.clone());
					let _ = app_handle.get_webview_window("main").unwrap().emit("applications", applications.clone());
				}
				win.app_name
			} else {
				String::new()
			};

			if app_name != previous {
				let application_profiles = &APPLICATION_PROFILES.read().await.value;
				let application = application_profiles.get(&app_name);
				let default = application_profiles.get("opendeck_default");
				for value in crate::shared::DEVICES.iter() {
					let device = value.key();
					let Some(profile) = application.and_then(|d| d.get(device)).or(default.and_then(|d| d.get(device))) else {
						continue;
					};
					if crate::store::profiles::DEVICE_STORES.write().await.get_selected_profile(device).ok().as_ref() == Some(profile) {
						continue;
					}
					let _ = app_handle.get_webview_window("main").unwrap().emit(
						"switch_profile",
						SwitchProfileEvent {
							device: device.clone(),
							profile: profile.clone(),
						},
					);
				}
				previous = app_name;
			}

			tokio::time::sleep(std::time::Duration::from_millis(250)).await;
		}
	});

	tokio::spawn(async move {
		let mut system = System::new_with_specifics(RefreshKind::nothing().with_processes(ProcessRefreshKind::nothing().without_tasks()));

		loop {
			if !APPLICATION_PLUGINS.read().await.is_empty() {
				system.refresh_processes_specifics(ProcessesToUpdate::All, true, ProcessRefreshKind::nothing().without_tasks());

				for (application, processes) in APPLICATION_PROCESSES.write().await.iter_mut() {
					let mut alive_processes = Vec::with_capacity(processes.len());
					for pid in processes.iter() {
						if system.process(Pid::from_u32(*pid)).is_some() {
							alive_processes.push(*pid);
						} else {
							for plugin in APPLICATION_PLUGINS.read().await.get(application).into_iter().flatten() {
								let _ = crate::events::outbound::applications::application_did_terminate(plugin, application.clone()).await;
							}
						}
					}
					*processes = alive_processes;
				}
			}

			let application_plugins = APPLICATION_PLUGINS.read().await;
			for (application, plugins) in application_plugins.iter() {
				for process in system.processes_by_exact_name(application.as_ref()) {
					let pid = process.pid().as_u32();
					let mut application_processes = APPLICATION_PROCESSES.write().await;
					let pids = application_processes.entry(application.clone()).or_default();
					if !pids.contains(&pid) {
						pids.push(pid);
						for plugin in plugins {
							let _ = crate::events::outbound::applications::application_did_launch(plugin, application.clone()).await;
						}
					}
				}
			}
			drop(application_plugins);

			tokio::time::sleep(std::time::Duration::from_millis(500)).await;
		}
	});
}

pub async fn start_monitoring(plugin: &str, applications: &Vec<String>) {
	let mut application_plugins = APPLICATION_PLUGINS.write().await;

	for application in applications {
		application_plugins.entry(application.to_owned()).or_default().push(plugin.to_owned());

		let application_processes = APPLICATION_PROCESSES.read().await;
		if let Some(pids) = application_processes.get(application) {
			for _ in pids {
				let _ = crate::events::outbound::applications::application_did_launch(plugin, application.to_owned()).await;
			}
		}
	}
}

pub async fn stop_monitoring(plugin: &str) {
	let mut application_plugins = APPLICATION_PLUGINS.write().await;
	for plugins in application_plugins.values_mut() {
		plugins.retain(|p| p != plugin);
	}
	application_plugins.retain(|_, p| !p.is_empty());
}
