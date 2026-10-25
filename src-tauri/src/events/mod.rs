pub mod frontend;
pub mod inbound;
pub mod outbound;

use inbound::RegisterEvent;

use std::collections::HashMap;
use std::sync::LazyLock;

use futures::{SinkExt, StreamExt, stream::SplitSink};
use tokio::net::TcpStream;
use tokio::sync::mpsc::{UnboundedSender, unbounded_channel};
use tokio::sync::{Mutex, RwLock};
use tokio_tungstenite::{WebSocketStream, tungstenite::Message};

type Sockets = LazyLock<Mutex<HashMap<String, UnboundedSender<Message>>>>;
static PLUGIN_SOCKETS: Sockets = LazyLock::new(|| Mutex::new(HashMap::new()));
static PROPERTY_INSPECTOR_SOCKETS: Sockets = LazyLock::new(|| Mutex::new(HashMap::new()));
static PLUGIN_QUEUES: LazyLock<RwLock<HashMap<String, Vec<Message>>>> = LazyLock::new(|| RwLock::new(HashMap::new()));
static PROPERTY_INSPECTOR_QUEUES: LazyLock<RwLock<HashMap<String, Vec<Message>>>> = LazyLock::new(|| RwLock::new(HashMap::new()));

pub async fn registered_plugins() -> Vec<String> {
	PLUGIN_SOCKETS.lock().await.keys().map(|x| x.to_owned()).collect()
}

/// Spawn a task that writes messages to a WebSocket in order.
/// Senders never wait on the socket, so a plugin that stops reading cannot block events to other plugins.
fn spawn_writer(mut sink: SplitSink<WebSocketStream<TcpStream>, Message>) -> UnboundedSender<Message> {
	let (tx, mut rx) = unbounded_channel::<Message>();
	tokio::spawn(async move {
		while let Some(message) = rx.recv().await {
			if sink.send(message).await.is_err() {
				break;
			}
		}
	});
	tx
}

/// Register a plugin or property inspector to send and receive events with its WebSocket.
pub async fn register_plugin(event: RegisterEvent, stream: WebSocketStream<TcpStream>) {
	let (read, write) = stream.split();
	let sender = spawn_writer(read);
	match event {
		RegisterEvent::RegisterPlugin { uuid } => {
			{
				// Hold the sockets lock while draining the queue so that no message is queued after the drain but before the insert.
				let mut sockets = PLUGIN_SOCKETS.lock().await;
				if sockets.contains_key(&uuid) {
					log::error!("Plugin {} is already registered; ignoring duplicate registration", uuid);
					return;
				}
				log::debug!("Registered plugin {}", uuid);
				if let Some(queue) = PLUGIN_QUEUES.write().await.remove(&uuid) {
					for message in queue {
						let _ = sender.send(message);
					}
				}
				sockets.insert(uuid.clone(), sender);
			}
			tokio::spawn(async move {
				let uuid = uuid;
				write.for_each(|event| inbound::process_incoming_message(event, &uuid, false)).await;
				PLUGIN_SOCKETS.lock().await.remove(&uuid);
			});
		}
		RegisterEvent::RegisterPropertyInspector { uuid } => {
			{
				let mut sockets = PROPERTY_INSPECTOR_SOCKETS.lock().await;
				if sockets.contains_key(&uuid) {
					return;
				}
				if let Some(queue) = PROPERTY_INSPECTOR_QUEUES.write().await.remove(&uuid) {
					for message in queue {
						let _ = sender.send(message);
					}
				}
				sockets.insert(uuid.clone(), sender);
			}
			tokio::spawn(async move {
				let uuid = uuid;
				write.for_each(|event| inbound::process_incoming_message_pi(event, &uuid)).await;
				PROPERTY_INSPECTOR_SOCKETS.lock().await.remove(&uuid);
			});
		}
	};
}
