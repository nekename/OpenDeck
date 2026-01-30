use openaction::*;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Default, Clone)]
#[serde(default)]
pub struct OpenUrlSettings {
	down: Option<String>,
	up: Option<String>,
	anticlockwise: Option<String>,
	clockwise: Option<String>,
}

pub struct OpenUrlAction;
#[async_trait]
impl Action for OpenUrlAction {
	const UUID: &'static str = "com.amansprojects.starterpack.openurl";
	type Settings = OpenUrlSettings;

	async fn key_down(
		&self,
		_instance: &Instance,
		settings: &Self::Settings,
	) -> OpenActionResult<()> {
		if let Some(url) = settings.down.as_ref()
			&& !url.trim().is_empty()
		{
			open_url(url.clone()).await?;
		}
		Ok(())
	}

	async fn key_up(
		&self,
		_instance: &Instance,
		settings: &Self::Settings,
	) -> OpenActionResult<()> {
		if let Some(url) = settings.up.as_ref()
			&& !url.trim().is_empty()
		{
			open_url(url.clone()).await?;
		}
		Ok(())
	}

	async fn dial_down(
		&self,
		instance: &Instance,
		settings: &Self::Settings,
	) -> OpenActionResult<()> {
		self.key_down(instance, settings).await
	}

	async fn dial_up(
		&self,
		instance: &Instance,
		settings: &Self::Settings,
	) -> OpenActionResult<()> {
		self.key_up(instance, settings).await
	}

	async fn dial_rotate(
		&self,
		_instance: &Instance,
		settings: &Self::Settings,
		ticks: i16,
		_pressed: bool,
	) -> OpenActionResult<()> {
		let input = if ticks < 0 {
			&settings.anticlockwise
		} else {
			&settings.clockwise
		};
		if let Some(url) = input.as_ref()
			&& !url.trim().is_empty()
		{
			open_url(url.clone()).await?;
		}
		Ok(())
	}
}
