use crate::iconpacks::manager::IconPackManager;
use tauri::http;
use tauri::http::{header::*, response::Builder as ResponseBuilder, status::StatusCode};
use tauri::{AppHandle, Manager};

pub fn iconpack_access_protocol(app: AppHandle, request: http::Request<Vec<u8>>) -> Result<http::Response<Vec<u8>>, Box<dyn std::error::Error>> {
	// let icon_packs = app.path().app_config_dir().unwrap().join("icon_packs");
	let manager = app.state::<IconPackManager>();

	// uri is like: iconpack://pack_id/icon_name
	if let Some((pack_id, icon_name)) = urlencoding::decode(request.uri().path()).unwrap().into_owned().trim_start_matches('/').split_once('/') {

        if let Some(icon_path) = manager.get_icon_path(pack_id, icon_name) {
			let ext = icon_path.extension().unwrap().to_string_lossy().to_string();
			let data = std::fs::read(&icon_path)?;
			// TODO: extract or find content type properly
			let content_type = match ext.as_str() {
				"svg" => "image/svg+xml",
				"png" => "image/png",
				"jpg" | "jpeg" => "image/jpeg",
				_ => "application/octet-stream",
			};
			let response = ResponseBuilder::new().status(StatusCode::OK).header(CONTENT_TYPE, content_type).body(data)?;
			return Ok(response);
		}
	}

	Err(Box::new(std::io::Error::new(std::io::ErrorKind::NotFound, "Icon not found")))
}
