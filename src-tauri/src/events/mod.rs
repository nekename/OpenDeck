pub mod frontend;
pub mod inbound;
pub mod outbound;

use inbound::RegisterEvent;

use std::collections::HashMap;

use futures::{SinkExt, StreamExt, stream::SplitSink};
use once_cell::sync::Lazy;
use tokio::net::TcpStream;
use tokio::sync::{Mutex, RwLock};
use tokio_tungstenite::{WebSocketStream, tungstenite::Message};

type Sockets = Lazy<Mutex<HashMap<String, SplitSink<WebSocketStream<TcpStream>, Message>>>>;
static PLUGIN_SOCKETS: Sockets = Lazy::new(|| Mutex::new(HashMap::new()));
static PROPERTY_INSPECTOR_SOCKETS: Sockets = Lazy::new(|| Mutex::new(HashMap::new()));
static PLUGIN_QUEUES: Lazy<RwLock<HashMap<String, Vec<Message>>>> = Lazy::new(|| RwLock::new(HashMap::new()));
static PROPERTY_INSPECTOR_QUEUES: Lazy<RwLock<HashMap<String, Vec<Message>>>> = Lazy::new(|| RwLock::new(HashMap::new()));

pub async fn registered_plugins() -> Vec<String> {
	PLUGIN_SOCKETS.lock().await.keys().map(|x| x.to_owned()).collect()
}

/// Register a plugin or property inspector to send and receive events with its WebSocket.
pub async fn register_plugin(event: RegisterEvent, stream: WebSocketStream<TcpStream>) {
	let (mut read, write) = stream.split();
	match event {
		RegisterEvent::RegisterPlugin { uuid } => {
			log::debug!("Registered plugin {}", uuid);
			if let Some(queue) = PLUGIN_QUEUES.read().await.get(&uuid) {
				for message in queue {
					let _ = read.feed(message.clone()).await;
				}
				let _ = read.flush().await;
			}
			PLUGIN_SOCKETS.lock().await.insert(uuid.clone(), read);
			tokio::spawn(async move {
				let uuid = uuid;
				write.for_each(|event| inbound::process_incoming_message(event, &uuid, false)).await;
				PLUGIN_SOCKETS.lock().await.remove(&uuid);
			});
		}
		RegisterEvent::RegisterPropertyInspector { uuid } => {
			if let Some(queue) = PROPERTY_INSPECTOR_QUEUES.read().await.get(&uuid) {
				for message in queue {
					let _ = read.feed(message.clone()).await;
				}
				let _ = read.flush().await;
			}
			PROPERTY_INSPECTOR_SOCKETS.lock().await.insert(uuid.clone(), read);
			tokio::spawn(async move {
				let uuid = uuid;
				write.for_each(|event| inbound::process_incoming_message_pi(event, &uuid)).await;
				PROPERTY_INSPECTOR_SOCKETS.lock().await.remove(&uuid);
			});
		}
	};
}
