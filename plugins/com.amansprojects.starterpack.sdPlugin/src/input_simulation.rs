use openaction::*;

use std::sync::LazyLock;

use enigo::{
	Enigo, Settings,
	agent::{Agent, Token},
};
use tokio::sync::Mutex;

static ENIGO: LazyLock<Mutex<Option<Enigo>>> = LazyLock::new(|| Mutex::new(Option::None));

pub async fn down_up(event: KeyEvent, action: &str) -> EventHandlerResult {
	if let Some(value) = event.payload.settings.as_object().unwrap().get(action) {
		let value = value.as_str().unwrap().to_owned();
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
		.unwrap()?;
	}

	Ok(())
}
