use serde::Serialize;

// Structs that make up the Info parameter passed to plugins and property inspectors during the registration procedure.

#[allow(non_snake_case)]
#[derive(Serialize)]
pub struct ApplicationInfo {
	pub font: String,
	pub language: String,
	pub platform: String,
	pub platformVersion: String,
	pub version: String,
}

#[derive(Serialize)]
pub struct PluginInfo {
	pub uuid: String,
	pub version: String,
}

#[allow(non_snake_case)]
#[derive(Serialize)]
pub struct ColoursInfo {
	pub buttonPressedBackgroundColor: String,
	pub buttonPressedBorderColor: String,
	pub buttonPressedTextColor: String,
	pub disabledColor: String,
	pub highlightColor: String,
	pub mouseDownColor: String,
}

#[derive(Clone, Serialize)]
pub struct DeviceSizeInfo {
	pub rows: u8,
	pub columns: u8,
}

#[derive(Clone, Serialize)]
pub struct DeviceInfo {
	pub id: String,
	pub name: String,
	pub size: DeviceSizeInfo,
	pub r#type: u8,
}

impl From<&crate::shared::DeviceInfo> for DeviceInfo {
	fn from(device: &crate::shared::DeviceInfo) -> DeviceInfo {
		DeviceInfo {
			id: device.id.clone(),
			name: device.name.clone(),
			size: DeviceSizeInfo {
				rows: device.rows,
				columns: device.columns,
			},
			r#type: device.r#type,
		}
	}
}

#[allow(non_snake_case)]
#[derive(Serialize)]
pub struct Info {
	pub application: ApplicationInfo,
	pub plugin: PluginInfo,
	pub devicePixelRatio: u8,
	pub colors: ColoursInfo,
	pub devices: Vec<DeviceInfo>,
}

/// Construct the info parameter for a given plugin's UUID and version.
pub async fn make_info(uuid: String, version: String, wine: bool) -> Info {
	#[cfg(target_os = "windows")]
	let platform = "windows";
	#[cfg(target_os = "macos")]
	let platform = "mac";
	#[cfg(target_os = "linux")]
	let platform = "linux";

	Info {
		application: ApplicationInfo {
			font: "ui-sans-serif".to_owned(),
			language: crate::store::get_settings().unwrap().value.language,
			platform: if !wine { platform.to_owned() } else { "windows".to_owned() },
			platformVersion: if !wine { os_info::get().version().to_string() } else { "10.0.19045.4474".to_owned() },
			version: "7.0.0".to_owned(),
		},
		plugin: PluginInfo { uuid, version },
		devicePixelRatio: 0,
		colors: ColoursInfo {
			buttonPressedBackgroundColor: "#303030FF".to_owned(),
			buttonPressedBorderColor: "#646464FF".to_owned(),
			buttonPressedTextColor: "#969696FF".to_owned(),
			disabledColor: "#F7821B59".to_owned(),
			highlightColor: "#F7821BFF".to_owned(),
			mouseDownColor: "#CF6304FF".to_owned(),
		},
		devices: crate::shared::DEVICES.iter().map(|v| (&*v).into()).collect(),
	}
}
