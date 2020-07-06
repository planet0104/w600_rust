use futures_channel::mpsc::{unbounded, UnboundedSender};
use futures_util::{future, pin_mut, stream::TryStreamExt, StreamExt};
use serde_json::Value;
use std::{
    collections::HashMap,
    io::Error,
    sync::{Arc, Mutex},
};
use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::tungstenite::Message;

#[tokio::main]
async fn main() -> Result<(), Error> {
    // Create the event loop and TCP listener we'll accept connections on.
    let try_socket = TcpListener::bind("0.0.0.0:9001").await;
    let mut listener = try_socket.expect("Failed to bind");
    println!("Listening on: {:?}", listener.local_addr());

    let state = Arc::new(Mutex::new(HashMap::new()));

    while let Ok((stream, _)) = listener.accept().await {
        tokio::spawn(accept_connection(stream, state.clone()));
    }

    Ok(())
}

fn get_as_u8(map: &serde_json::map::Map<String, Value>, key: &str) -> u8 {
    map.get(key)
        .unwrap_or(&Value::String(String::from("0")))
        .as_str()
        .unwrap_or("0")
        .parse::<u8>()
        .unwrap_or(0)
}

async fn accept_connection(
    stream: TcpStream,
    peer_map: Arc<Mutex<HashMap<String, UnboundedSender<Message>>>>,
) {
    let addr = stream
        .peer_addr()
        .expect("connected streams should have a peer address");
    println!("Peer address: {}", addr);
    let local_addr = format!("{:?}", stream.local_addr());

    let ws_stream = tokio_tungstenite::accept_async(stream)
        .await
        .expect("Error during the websocket handshake occurred");

    println!("New WebSocket connection: {}", addr);

    let (tx, rx) = unbounded();
    peer_map.lock().unwrap().insert(local_addr.clone(), tx);
    let mut client_id = None;

    let (outgoing, incoming) = ws_stream.split();

    let broadcast_incoming = incoming.try_for_each(|msg| {
        println!("{:?}", msg);
        if let Message::Text(text) = msg {
            let mut peers = peer_map.lock().unwrap();
            if client_id.is_none() {
                let tx = peers.remove(&local_addr).unwrap();
                peers.insert(text.clone(), tx);
                println!("{} registered.", text);
                client_id = Some(text);
            } else {
                //转发消息
                if let Ok(Value::Object(map)) = serde_json::from_str::<Value>(&text) {
                    if let Some(Value::String(target)) = map.get("target") {
                        if let Some(tx) = peers.get_mut(target) {
                            let msg = Message::Binary(vec![
                                get_as_u8(&map, "r"),
                                get_as_u8(&map, "g"),
                                get_as_u8(&map, "b"),
                            ]);
                            println!("send {:?} to {}", msg, target);
                            tx.unbounded_send(msg).unwrap();
                        }
                    }
                }
            }
        }
        future::ok(())
    });

    let receive_from_others = rx.map(Ok).forward(outgoing);

    pin_mut!(broadcast_incoming, receive_from_others);
    future::select(broadcast_incoming, receive_from_others).await;

    println!("{:?} disconnected", client_id);
    if let Some(client_id) = client_id {
        peer_map.lock().unwrap().remove(&client_id);
    }
}
