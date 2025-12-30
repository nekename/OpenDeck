use std::sync::LazyLock;

use openaction::*;

use enigo::{
	Enigo, Settings,
	agent::{Agent, Token},
};
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

static ENIGO: LazyLock<Mutex<Option<Enigo>>> = LazyLock::new(|| Mutex::new(Option::None));

async fn execute_input(value: Option<String>) -> Result<(), anyhow::Error> {
	let Some(value) = value else {
		return Ok(());
	};
	if value.trim().is_empty() {
		return Ok(());
	}

	let mut enigo_guard = ENIGO.lock().await;
	std::thread::spawn(move || -> Result<(), anyhow::Error> {
		if enigo_guard.is_none() {
			enigo_guard.replace(Enigo::new(&Settings::default())?);
		}
		let enigo = enigo_guard.as_mut().unwrap();
		let tokens: Vec<Token> = ron::from_str(&value)?;
		for token in tokens {
			enigo.execute(&token).unwrap();
		}
		Ok(())
	})
	.join()
	.unwrap_or(Ok(()))?;

	Ok(())
}

#[derive(Serialize, Deserialize, Default, Clone)]
#[serde(default)]
pub struct InputSimulationSettings {
	down: Option<String>,
	up: Option<String>,
	anticlockwise: Option<String>,
	clockwise: Option<String>,
}

pub struct InputSimulationAction;
#[async_trait]
impl Action for InputSimulationAction {
	const UUID: &'static str = "com.amansprojects.starterpack.inputsimulation";
	type Settings = InputSimulationSettings;

	async fn key_down(
		&self,
		_instance: &Instance,
		settings: &Self::Settings,
	) -> OpenActionResult<()> {
		if let Err(error) = execute_input(settings.down.clone()).await {
			log::warn!("Failed to simulate input: {error}");
		}
		Ok(())
	}

	async fn key_up(
		&self,
		_instance: &Instance,
		settings: &Self::Settings,
	) -> OpenActionResult<()> {
		if let Err(error) = execute_input(settings.up.clone()).await {
			log::warn!("Failed to simulate input: {error}");
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
		for _ in 0..ticks.abs() {
			if let Err(error) = execute_input(if ticks < 0 {
				settings.anticlockwise.clone()
			} else {
				settings.clockwise.clone()
			})
			.await
			{
				log::warn!("Failed to simulate input: {error}");
			}
		}
		Ok(())
	}
}
