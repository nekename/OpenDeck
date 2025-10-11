use super::ActionEvent;

use std::sync::LazyLock;

use openaction::*;

use enigo::{
	Enigo, Settings,
	agent::{Agent, Token},
};
use tokio::sync::Mutex;

static ENIGO: LazyLock<Mutex<Option<Enigo>>> = LazyLock::new(|| Mutex::new(Option::None));

async fn execute_input(event: impl ActionEvent, action: &str) -> EventHandlerResult {
	let Some(settings) = event.settings().as_object() else {
		return Ok(());
	};
	let Some(value) = settings
		.get(action)
		.and_then(|v| v.as_str())
		.map(|x| x.to_owned())
	else {
		return Ok(());
	};
	if value.trim().is_empty() {
		return Ok(());
	}

	let mut enigo_guard = ENIGO.lock().await;
	std::thread::spawn(move || -> EventHandlerResult {
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

pub async fn down_up(action: &str, event: impl ActionEvent) -> EventHandlerResult {
	execute_input(event, action).await
}

pub async fn rotate(event: DialRotateEvent) -> EventHandlerResult {
	let ticks = event.payload.ticks;
	for _ in 0..ticks.abs() {
		if ticks < 0 {
			execute_input(event.clone(), "anticlockwise").await?;
		} else if ticks > 0 {
			execute_input(event.clone(), "clockwise").await?;
		}
	}
	Ok(())
}
