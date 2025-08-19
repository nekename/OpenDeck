use std::collections::HashMap;

use crate::shared::Action;

use serde::Deserialize;
use serde_inline_default::serde_inline_default;

#[derive(Deserialize)]
pub struct OS {
	#[serde(alias = "Platform")]
	pub platform: String,
}

#[allow(dead_code)]
#[serde_inline_default]
#[derive(Deserialize)]
pub struct PluginManifest {
	#[serde(alias = "Name")]
	pub name: String,

	#[serde(alias = "Author")]
	pub author: String,

	#[serde(alias = "Version")]
	pub version: String,

	#[serde(alias = "Icon")]
	pub icon: String,

	#[serde_inline_default("Custom".to_owned())]
	#[serde(alias = "Category")]
	pub category: String,

	#[serde(alias = "CategoryIcon")]
	pub category_icon: Option<String>,

	#[serde(alias = "Actions")]
	pub actions: Vec<Action>,

	#[serde(alias = "OS")]
	pub os: Vec<OS>,

	#[serde(alias = "CodePath")]
	pub code_path: Option<String>,

	#[serde(alias = "CodePathWin")]
	pub code_path_windows: Option<String>,

	#[serde(alias = "CodePathMac")]
	pub code_path_macos: Option<String>,

	#[serde(alias = "CodePathLin")]
	pub code_path_linux: Option<String>,

	#[serde(alias = "PropertyInspectorPath")]
	pub property_inspector_path: Option<String>,

	#[serde(alias = "DeviceNamespace")]
	pub device_namespace: Option<String>,

	#[serde(alias = "ApplicationsToMonitor")]
	pub applications_to_monitor: Option<HashMap<String, Vec<String>>>,

	#[serde(alias = "HasSettingsInterface")]
	pub has_settings_interface: Option<bool>,
}

pub fn read_manifest(base_path: &std::path::Path) -> Result<PluginManifest, anyhow::Error> {
	use anyhow::Context;

	let mut manifest: serde_json::Value = serde_json::from_str(
		std::fs::read_to_string(base_path.join("manifest.json"))
			.context("failed to read manifest")?
			.trim_start_matches("\u{feff}"),
	)
	.context("failed to parse manifest")?;

	let platform_overrides_path = base_path.join(format!("manifest.{}.json", std::env::consts::OS));
	if platform_overrides_path.exists() {
		if let Ok(Ok(platform_overrides)) = std::fs::read(platform_overrides_path).map(|v| serde_json::from_slice(&v)) {
			json_patch::merge(&mut manifest, &platform_overrides);
		}
	}

	serde_json::from_value(manifest).context("failed to parse manifest")
}
