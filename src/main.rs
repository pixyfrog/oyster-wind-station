mod rfm95w;

use axum::{routing::get, Router, extract::State};
use std::sync::{Arc, Mutex};

#[derive(Clone)]
struct AppState {
    packet: Arc<Mutex<rfm95w::WindPacket>>,
}

#[tokio::main]
async fn main() {
    let packet = rfm95w::read_packet_mock();
    let state = AppState {
        packet: Arc::new(Mutex::new(packet)),
    };

    let app = Router::new()
        .route("/", get(handler))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    println!("Server running on http://0.0.0.0:3000");
    axum::serve(listener, app).await.unwrap();
}

async fn handler(State(state): State<AppState>) -> String {
    let packet = state.packet.lock().unwrap();
    format!("Node: {}, Wind: {} (x0.1 m/s), Battery: {} mV, Seq: {}",
        packet.node_id, packet.wind_speed, packet.battery_mv, packet.sequence)
}