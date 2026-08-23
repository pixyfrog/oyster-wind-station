mod packet;
mod rfm95w;


use axum::{routing::get, Router, extract::State};

use std::sync::{Arc, Mutex};

#[derive(Clone)]
struct AppState {
    packet: Arc<Mutex<Option<packet::WindPacket>>>,
}

#[tokio::main]
async fn main() {
    match rfm95w::get_version() {
        Ok(v) => println!("Radio version: 0x{:2X}", v),
        Err(e) => println!("Radio error: {:?}", e),
    }
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
                wind_speed: 177,
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
        Some(ref p) => format!("Node: {}, Wind: {} (m/s), Battery: {} mV, Seq: {}",
            p.node_id, (p.wind_speed as f32) / 10.0, p.battery_mv, p.sequence),
        None => "Station Starting up".to_string(),
    }
}