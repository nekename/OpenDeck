use crate::store::{NotProfile, Store};

use std::collections::HashMap;

use active_win_pos_rs::get_active_window;
use once_cell::sync::Lazy;
use sysinfo::{Pid, ProcessRefreshKind, System};
use tauri::{Emitter, Manager};
use tokio::sync::RwLock;

pub type ApplicationProfiles = HashMap<String, HashMap<String, String>>;
impl NotProfile for ApplicationProfiles {}

pub static APPLICATIONS: RwLock<Vec<String>> = RwLock::const_new(Vec::new());
pub static APPLICATION_PROFILES: Lazy<RwLock<Store<ApplicationProfiles>>> = Lazy::new(|| RwLock::const_new(Store::new("applications", &crate::shared::config_dir(), HashMap::new()).unwrap()));

pub static MONITORED_PROCESSES: Lazy<RwLock<HashMap<String, Vec<u32>>>> = Lazy::new(|| RwLock::const_new(HashMap::new()));
pub static MONITORED_APPLICATIONS: Lazy<RwLock<HashMap<String, Vec<String>>>> = Lazy::new(|| RwLock::const_new(HashMap::new()));

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
				if !applications.contains(&win.app_name) && !win.app_name.to_lowercase().starts_with("opendeck") && !win.app_name.trim().is_empty() {
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
		log::debug!("Starting application monitor...");
		let mut system = System::new_all();

		loop {
			system.refresh_processes_specifics(sysinfo::ProcessesToUpdate::All, true, ProcessRefreshKind::everything().without_tasks());

			// Check for terminated processes
			for (app_name, processes) in MONITORED_PROCESSES.write().await.iter_mut() {
				let mut new_processes = Vec::with_capacity(processes.len());
				for pid in processes.iter() {
					if system.process(Pid::from_u32(*pid)).is_some() {
						new_processes.push(*pid);
						continue;
					}

					log::debug!("Process {} with PID {} has terminated", app_name, pid);
					for plugin in MONITORED_APPLICATIONS.read().await.get(app_name).into_iter().flatten() {
						let _ = crate::events::outbound::app_monitor::application_did_terminate(plugin, app_name.clone()).await;
					}
				}
				*processes = new_processes;
			}

			// Check for new processes
			let monitoring_plugins = MONITORED_APPLICATIONS.read().await;
			for (app_name, plugins) in &*monitoring_plugins {
				for process in system.processes_by_exact_name(app_name.as_ref()) {
					let pid = process.pid().as_u32();

					let mut monitored_apps = MONITORED_PROCESSES.write().await;
					let pids = monitored_apps.entry(app_name.clone()).or_default();
					if !pids.contains(&pid) {
						log::debug!("Found new process: {} with PID {}", &app_name, pid);
						pids.push(pid);

						for plugin in plugins {
							let _ = crate::events::outbound::app_monitor::application_did_launch(plugin, app_name.to_string()).await;
						}
					}
				}
			}

			tokio::time::sleep(std::time::Duration::from_millis(250)).await;
		}
	});
}

pub async fn start_monitoring(plugin: &str, application: &str) -> Result<(), anyhow::Error> {
	let mut plugins = MONITORED_APPLICATIONS.write().await;
	plugins.entry(application.to_owned()).or_default().push(plugin.to_owned());
	drop(plugins);

	let monitored_apps = MONITORED_PROCESSES.read().await;
	if let Some(pids) = monitored_apps.get(application) {
		for _ in 0..pids.len() {
			let _ = crate::events::outbound::app_monitor::application_did_launch(plugin, application.to_owned()).await;
		}
	}
	log::debug!("{} is now being monitored by {}", application, plugin);
	Ok(())
}

pub async fn stop_monitoring_all(plugin: &str) -> Result<(), anyhow::Error> {
	let plugins = MONITORED_APPLICATIONS.read().await;
	for (application, plugins) in &*plugins {
		if plugins.contains(&plugin.to_string()) {
			stop_monitoring(plugin, application).await?;
		}
	}
	Ok(())
}

pub async fn stop_monitoring(plugin: &str, application: &str) -> Result<(), anyhow::Error> {
	let mut plugins = MONITORED_APPLICATIONS.write().await;
	let entry = plugins.entry(application.to_owned()).or_default();
	entry.retain(|p| p != plugin);

	if entry.is_empty() {
		MONITORED_PROCESSES.write().await.remove(application);
	}
	log::debug!("{} is no longer being monitored by {}", application, plugin);
	Ok(())
}
