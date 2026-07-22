mod packet;

use axum::{routing::get, Router, extract::State};
use std::sync::{Arc, Mutex};

#[derive(Clone)]
struct AppState {
    packet: Arc<Mutex<Option<packet::WindPacket>>>,
}

#[tokio::main]
async fn main() {
    let state = AppState {
        packet: Arc::new(Mutex::new(None)),
    };

    let state_for_task = state.clone();
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
            let mut packet = state_for_task.packet.lock().unwrap();
            *packet = Some(packet::WindPacket {
                node_id: 1,
                wind_speed: 152,
                battery_mv: 3700,
                sequence: 42,
            });
            println!("Packet updated");
        }
    });

    let app = Router::new()
        .route("/", get(handler))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    println!("Server on http://192.168.18.80:3000");

    axum::serve(listener, app).await.unwrap();
}

async fn handler(State(state): State<AppState>) -> String {
    let packet = state.packet.lock().unwrap();
    match *packet {
        Some(ref p) => format!("Node: {}, Wind: {} (x0.1 m/s), Battery: {} mV, Seq: {}",
            p.node_id, p.wind_speed, p.battery_mv, p.sequence),
        None => "No packet received yet".to_string(),
    }
}