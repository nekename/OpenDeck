pub mod info_param;
pub mod manifest;
mod webserver;

use crate::APP_HANDLE;
use crate::built_info::TARGET;
use crate::shared::{CATEGORIES, Category, config_dir, convert_icon, is_flatpak, log_dir};
use crate::store::get_settings;

use std::collections::HashMap;
use std::process::{Child, Command, Stdio};
use std::sync::LazyLock;
use std::{fs, path};

use tauri::{AppHandle, Manager};

use futures::StreamExt;
use tokio::net::{TcpListener, TcpStream};

use anyhow::anyhow;
use log::{error, warn};
use tokio::sync::{Mutex, RwLock};

enum PluginInstance {
	Webview,
	Wine(Child),
	Native(Child),
	Node(Child),
}

pub static DEVICE_NAMESPACES: LazyLock<RwLock<HashMap<String, String>>> = LazyLock::new(|| RwLock::new(HashMap::new()));
static INSTANCES: LazyLock<Mutex<HashMap<String, PluginInstance>>> = LazyLock::new(|| Mutex::new(HashMap::new()));

pub static PORT_BASE: LazyLock<u16> = LazyLock::new(|| {
	let mut base = 57116;
	loop {
		let websocket_result = std::net::TcpListener::bind(format!("0.0.0.0:{}", base));
		let webserver_result = std::net::TcpListener::bind(format!("0.0.0.0:{}", base + 2));
		if websocket_result.is_ok() && webserver_result.is_ok() {
			log::debug!("Using ports {} and {}", base, base + 2);
			break;
		}
		base += 1;
	}
	base
});

/// Initialise a plugin from a given directory.
pub async fn initialise_plugin(path: &path::Path) -> anyhow::Result<()> {
	let plugin_uuid = path.file_name().unwrap().to_str().unwrap();

	let mut manifest = manifest::read_manifest(path)?;

	if let Some(icon) = manifest.category_icon {
		let category_icon_path = path.join(icon);
		manifest.category_icon = Some(convert_icon(category_icon_path.to_string_lossy().to_string()));
	}

	for action in &mut manifest.actions {
		plugin_uuid.clone_into(&mut action.plugin);

		let action_icon_path = path.join(action.icon.clone());
		action.icon = convert_icon(action_icon_path.to_str().unwrap().to_owned());

		if !action.property_inspector.is_empty() {
			action.property_inspector = path.join(&action.property_inspector).to_string_lossy().to_string();
		} else if let Some(ref property_inspector) = manifest.property_inspector_path {
			action.property_inspector = path.join(property_inspector).to_string_lossy().to_string();
		}

		for state in &mut action.states {
			if state.image == "actionDefaultImage" {
				state.image.clone_from(&action.icon);
			} else {
				let state_icon = path.join(state.image.clone());
				state.image = convert_icon(state_icon.to_str().unwrap().to_owned());
			}

			match state.family.clone().to_lowercase().trim() {
				"arial" => "Liberation Sans",
				"arial black" => "Archivo Black",
				"comic sans ms" => "Comic Neue",
				"courier" | "Courier New" => "Courier Prime",
				"georgia" => "Tinos",
				"impact" => "Anton",
				"microsoft sans serif" | "Times New Roman" => "Liberation Serif",
				"tahoma" | "Verdana" => "Open Sans",
				"trebuchet ms" => "Fira Sans",
				_ => continue,
			}
			.clone_into(&mut state.family);
		}
	}

	{
		let mut categories = CATEGORIES.write().await;
		if let Some(category) = categories.get_mut(&manifest.category) {
			for action in manifest.actions {
				if let Some(index) = category.actions.iter().position(|v| v.uuid == action.uuid) {
					category.actions.remove(index);
				}
				category.actions.push(action);
			}
		} else {
			let mut category: Category = Category {
				icon: manifest.category_icon,
				actions: vec![],
			};
			for action in manifest.actions {
				category.actions.push(action);
			}
			if !category.actions.is_empty() {
				categories.insert(manifest.category, category);
			}
		}
	}

	if let Some(namespace) = manifest.device_namespace {
		DEVICE_NAMESPACES.write().await.insert(namespace, plugin_uuid.to_owned());
	}

	#[cfg(target_os = "windows")]
	let platform = "windows";
	#[cfg(target_os = "macos")]
	let platform = "mac";
	#[cfg(target_os = "linux")]
	let platform = "linux";

	let mut code_path = manifest.code_path;
	let mut use_wine = false;
	let mut supported = false;

	// Determine the method used to run the plugin based on its supported operating systems and the current operating system.
	for os in manifest.os {
		if os.platform == platform {
			#[cfg(target_os = "windows")]
			if manifest.code_path_windows.is_some() {
				code_path = manifest.code_path_windows.clone();
			}
			#[cfg(target_os = "macos")]
			if manifest.code_path_macos.is_some() {
				code_path = manifest.code_path_macos;
			}
			#[cfg(target_os = "linux")]
			if manifest.code_path_linux.is_some() {
				code_path = manifest.code_path_linux;
			}
			code_path = manifest.code_paths.and_then(|p| p.get(TARGET).cloned()).or(code_path);

			use_wine = false;

			supported = true;
			break;
		} else if os.platform == "windows" {
			use_wine = true;
			supported = true;
		}
	}

	if code_path.is_none() && use_wine {
		code_path = manifest.code_path_windows;
	}

	if !supported || code_path.is_none() {
		return Err(anyhow!("unsupported on platform {}", platform));
	}

	let code_path = code_path.unwrap();
	let port_string = PORT_BASE.to_string();
	let args = ["-port", port_string.as_str(), "-pluginUUID", plugin_uuid, "-registerEvent", "registerPlugin", "-info"];

	if code_path.to_lowercase().ends_with(".html") || code_path.to_lowercase().ends_with(".htm") || code_path.to_lowercase().ends_with(".xhtml") {
		let url = format!("http://localhost:{}/", *PORT_BASE + 2) + path.join(code_path).to_str().unwrap();
		let window = tauri::WebviewWindowBuilder::new(APP_HANDLE.get().unwrap(), plugin_uuid.replace('.', "_"), tauri::WebviewUrl::External(url.parse()?))
			.title(plugin_uuid)
			.visible(false)
			.build()?;

		if fs::exists(path.join("debug")).unwrap_or(false) {
			let _ = window.show();
			window.open_devtools();
		}

		let info = info_param::make_info(plugin_uuid.to_owned(), manifest.version, false).await;
		window.eval(format!(
			r#"const opendeckInit = () => {{
				try {{
					if (document.readyState !== "complete") throw new Error("not ready");
					if (typeof connectOpenActionSocket === "function") connectOpenActionSocket({port}, "{uuid}", "{event}", `{info}`);
					else connectElgatoStreamDeckSocket({port}, "{uuid}", "{event}", `{info}`);
				}} catch (e) {{
					setTimeout(opendeckInit, 10);
				}}
			}};
			opendeckInit();
			"#,
			port = *PORT_BASE,
			uuid = plugin_uuid,
			event = "registerPlugin",
			info = serde_json::to_string(&info)?
		))?;

		INSTANCES.lock().await.insert(plugin_uuid.to_owned(), PluginInstance::Webview);
	} else if code_path.to_lowercase().ends_with(".js") || code_path.to_lowercase().ends_with(".mjs") || code_path.to_lowercase().ends_with(".cjs") {
		// Check for Node.js installation and version in one go.
		let command = if is_flatpak() { "flatpak-spawn" } else { "node" };
		let extra_args = if is_flatpak() { vec!["--host", "node"] } else { vec![] };
		let version_output = Command::new(command).args(&extra_args).arg("--version").output();
		if version_output.is_err() || String::from_utf8(version_output.unwrap().stdout).unwrap().trim() < "v20.0.0" {
			return Err(anyhow!("Node.js version 20.0.0 or higher is required"));
		}

		let info = info_param::make_info(plugin_uuid.to_owned(), manifest.version, true).await;
		let log_file = fs::File::create(log_dir().join("plugins").join(format!("{plugin_uuid}.log")))?;

		#[cfg(target_os = "windows")]
		{
			use std::os::windows::process::CommandExt;
			let child = Command::new(command)
				.current_dir(path)
				.args(extra_args)
				.arg(code_path)
				.args(args)
				.arg(serde_json::to_string(&info)?)
				.stdout(Stdio::from(log_file.try_clone()?))
				.stderr(Stdio::from(log_file))
				.creation_flags(0x08000000)
				.spawn()?;

			INSTANCES.lock().await.insert(plugin_uuid.to_owned(), PluginInstance::Node(child));
		}

		#[cfg(not(target_os = "windows"))]
		{
			let child = Command::new(command)
				.current_dir(path)
				.args(extra_args)
				.arg(code_path)
				.args(args)
				.arg(serde_json::to_string(&info)?)
				.stdout(Stdio::from(log_file.try_clone()?))
				.stderr(Stdio::from(log_file))
				.spawn()?;

			INSTANCES.lock().await.insert(plugin_uuid.to_owned(), PluginInstance::Node(child));
		}
	} else if use_wine {
		let command = if is_flatpak() { "flatpak-spawn" } else { "wine" };
		let extra_args = if is_flatpak() { vec!["--host", "wine"] } else { vec![] };
		let result = Command::new(command)
			.args(&extra_args)
			.arg("--version")
			.stdout(Stdio::null())
			.stderr(Stdio::null())
			.spawn()
			.and_then(|mut child| child.wait())
			.map(|status| status.success());
		if !matches!(result, Ok(true)) {
			return Err(anyhow!("failed to detect an installation of Wine"));
		}

		let info = info_param::make_info(plugin_uuid.to_owned(), manifest.version, true).await;
		let log_file = fs::File::create(log_dir().join("plugins").join(format!("{plugin_uuid}.log")))?;

		let mut command = Command::new(command);
		command
			.current_dir(path)
			.args(extra_args)
			.arg(code_path)
			.args(args)
			.arg(serde_json::to_string(&info)?)
			.stdout(Stdio::from(log_file.try_clone()?))
			.stderr(Stdio::from(log_file));
		if get_settings()?.value.separatewine {
			command.env("WINEPREFIX", path.join("wineprefix").to_string_lossy().to_string());
		} else {
			let _ = fs::remove_dir_all(path.join("wineprefix"));
		}
		let child = command.spawn()?;

		INSTANCES.lock().await.insert(plugin_uuid.to_owned(), PluginInstance::Wine(child));
	} else {
		let info = info_param::make_info(plugin_uuid.to_owned(), manifest.version, false).await;
		let log_file = fs::File::create(log_dir().join("plugins").join(format!("{plugin_uuid}.log")))?;

		#[cfg(target_os = "windows")]
		{
			use std::os::windows::process::CommandExt;
			let child = Command::new(path.join(code_path))
				.current_dir(path)
				.args(args)
				.arg(serde_json::to_string(&info)?)
				.stdout(Stdio::from(log_file.try_clone()?))
				.stderr(Stdio::from(log_file))
				.creation_flags(0x08000000)
				.spawn()?;

			INSTANCES.lock().await.insert(plugin_uuid.to_owned(), PluginInstance::Native(child));
		}

		#[cfg(unix)]
		{
			use std::os::unix::fs::PermissionsExt;
			fs::set_permissions(path.join(&code_path), fs::Permissions::from_mode(0o755))?;
		}

		#[cfg(not(target_os = "windows"))]
		{
			let child = Command::new(path.join(code_path))
				.current_dir(path)
				.args(args)
				.arg(serde_json::to_string(&info)?)
				.stdout(Stdio::from(log_file.try_clone()?))
				.stderr(Stdio::from(log_file))
				.spawn()?;

			INSTANCES.lock().await.insert(plugin_uuid.to_owned(), PluginInstance::Native(child));
		}
	}

	if let Some(applications) = manifest.applications_to_monitor
		&& let Some(applications) = applications.get(platform)
	{
		crate::application_watcher::start_monitoring(plugin_uuid, applications).await;
	}

	Ok(())
}

pub async fn deactivate_plugin(app: &AppHandle, uuid: &str) -> Result<(), anyhow::Error> {
	{
		let mut namespaces = DEVICE_NAMESPACES.write().await;
		if let Some((namespace, _)) = namespaces.clone().iter().find(|(_, plugin)| uuid == **plugin) {
			namespaces.remove(namespace);
			drop(namespaces);
			let devices = crate::shared::DEVICES.iter().map(|v| v.key().to_owned()).filter(|id| &id[..2] == namespace).collect::<Vec<_>>();
			for device in devices {
				crate::events::inbound::devices::deregister_device("", crate::events::inbound::PayloadEvent { payload: device }).await?;
			}
			crate::events::frontend::update_devices().await;
		}
	}

	crate::application_watcher::stop_monitoring(uuid).await;

	if let Some(instance) = INSTANCES.lock().await.remove(uuid) {
		match instance {
			PluginInstance::Webview => {
				if let Some(window) = app.get_webview_window(&uuid.replace('.', "_")) {
					window.close()?;
					tokio::time::sleep(std::time::Duration::from_millis(10)).await;
				}
			}
			PluginInstance::Node(mut child) | PluginInstance::Wine(mut child) | PluginInstance::Native(mut child) => {
				child.kill()?;
				child.wait()?;
			}
		}
		Ok(())
	} else {
		Err(anyhow!("instance of plugin {} not found", uuid))
	}
}

#[cfg(windows)]
pub async fn deactivate_plugins() {
	let uuids = {
		let instances = INSTANCES.lock().await;
		instances.keys().cloned().collect::<Vec<_>>()
	};

	let app = APP_HANDLE.get().unwrap();
	for uuid in uuids {
		let _ = deactivate_plugin(app, &uuid).await;
	}
}

/// Initialise plugins from the plugins directory.
pub fn initialise_plugins() {
	tokio::spawn(init_websocket_server());
	tokio::spawn(webserver::init_webserver(config_dir()));

	let plugin_dir = config_dir().join("plugins");
	let _ = fs::create_dir_all(&plugin_dir);
	let _ = fs::create_dir_all(log_dir().join("plugins"));

	if let Ok(Ok(entries)) = APP_HANDLE.get().unwrap().path().resolve("plugins", tauri::path::BaseDirectory::Resource).map(fs::read_dir) {
		for entry in entries.flatten() {
			if let Err(error) = (|| -> Result<(), anyhow::Error> {
				let builtin_version = semver::Version::parse(&serde_json::from_slice::<manifest::PluginManifest>(&fs::read(entry.path().join("manifest.json"))?)?.version)?;
				let existing_path = plugin_dir.join(entry.file_name());
				if (|| -> Result<(), anyhow::Error> {
					let existing_version = semver::Version::parse(&serde_json::from_slice::<manifest::PluginManifest>(&fs::read(existing_path.join("manifest.json"))?)?.version)?;
					if existing_version < builtin_version {
						Err(anyhow::anyhow!("builtin version is newer than existing version"))
					} else {
						Ok(())
					}
				})()
				.is_err()
				{
					if existing_path.exists() {
						fs::rename(&existing_path, existing_path.with_extension("old"))?;
					}
					if crate::shared::copy_dir(entry.path(), &existing_path).is_err() && existing_path.with_extension("old").exists() {
						fs::rename(existing_path.with_extension("old"), &existing_path)?;
					}
					let _ = fs::remove_dir_all(existing_path.with_extension("old"));
				}
				Ok(())
			})() {
				error!("Failed to upgrade builtin plugin {}: {}", entry.file_name().to_string_lossy(), error);
			}
		}
	}

	let entries = match fs::read_dir(&plugin_dir) {
		Ok(p) => p,
		Err(error) => {
			error!("Failed to read plugins directory at {}: {}", plugin_dir.display(), error);
			panic!()
		}
	};

	// Iterate through all directory entries in the plugins folder and initialise them as plugins if appropriate
	for entry in entries {
		if let Ok(entry) = entry {
			let path = match entry.metadata().unwrap().is_symlink() {
				true => entry.path().parent().unwrap_or_else(|| path::Path::new(".")).join(fs::read_link(entry.path()).unwrap()),
				false => entry.path(),
			};
			let metadata = fs::metadata(&path).unwrap();
			if metadata.is_dir() {
				tokio::spawn(async move {
					if let Err(error) = initialise_plugin(&path).await {
						warn!("Failed to initialise plugin at {}: {:#}", path.display(), error);
					}
				});
			}
		} else if let Err(error) = entry {
			warn!("Failed to read entry of plugins directory: {}", error)
		}
	}
}

/// Start the WebSocket server that plugins communicate with.
async fn init_websocket_server() {
	let listener = match TcpListener::bind(format!("0.0.0.0:{}", *PORT_BASE)).await {
		Ok(listener) => listener,
		Err(error) => {
			error!("Failed to bind plugin WebSocket server to socket: {}", error);
			return;
		}
	};

	#[cfg(windows)]
	{
		use std::os::windows::io::AsRawSocket;
		use windows_sys::Win32::Foundation::{HANDLE_FLAG_INHERIT, SetHandleInformation};

		unsafe { SetHandleInformation(listener.as_raw_socket() as _, HANDLE_FLAG_INHERIT, 0) };
	}

	while let Ok((stream, _)) = listener.accept().await {
		accept_connection(stream).await;
	}
}

/// Handle incoming data from a WebSocket connection.
async fn accept_connection(stream: TcpStream) {
	let mut socket = match tokio_tungstenite::accept_async(stream).await {
		Ok(socket) => socket,
		Err(error) => {
			warn!("Failed to complete WebSocket handshake: {}", error);
			return;
		}
	};

	let Ok(register_event) = socket.next().await.unwrap() else {
		return;
	};
	match serde_json::from_str(&register_event.clone().into_text().unwrap()) {
		Ok(event) => crate::events::register_plugin(event, socket).await,
		Err(_) => {
			let _ = crate::events::inbound::process_incoming_message(Ok(register_event), "", false).await;
		}
	}
}
