mod packet;
mod rfm95w;


use axum::{routing::get, Router, extract::State};
use std::time::Duration;
use std::sync::{Arc, Mutex};

#[derive(Clone)]
struct AppState {
    packet: Arc<Mutex<Option<packet::WindPacketV2>>>,
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
    
    let state_for_radio = state.clone();
    std::thread::spawn(move || start_radio_rx_task(state_for_radio));


     // Mock packet task disabled while testing real reception.   

    //let state_for_task = state.clone();
    //tokio::spawn(async move {
    //    loop {
    //        tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
    //        let mut packet = state_for_task.packet.lock().unwrap();
    //        *packet = Some(packet::WindPacket {
    //            node_id: 1,
    //            wind_speed: 177,
    //            battery_mv: 3700,
    //            sequence: 42,
    //        });
    //        println!("Packet updated");
    //    }
    //});

    let app = Router::new()
        .route("/", get(handler))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    println!("Server on http://192.168.18.80:3000");

    axum::serve(listener, app).await.unwrap();
}

fn start_radio_rx_task(state: AppState) {
    match rfm95w::Radio::new_receiver() {
        Ok(mut radio) => {
            println!("Radio RX task started");

            loop {
                match radio.poll_receive(Duration::from_millis(1000)) {
                    Ok(Some(bytes)) => {
                        println!("RX {} bytes: {:02X?}", bytes.len(), bytes);

                        match bytes.len() {
                            9 => println!(
                                "9-byte frame — v1 emitter, dropped ({} bytes)",
                                bytes.len()
                            ),
                            18 => match packet::decode_v2(&bytes) {
                                Some(p) => {
                                    println!(
                                        "DECODED v2 avg = {:.1}, gust = {:.1}, lull = {:.1}",
                                        (p.wind_avg as f32) / 10.0,
                                        (p.wind_gust as f32) / 10.0,
                                        (p.wind_lull as f32) / 10.0
                                    );
                                    let mut shared = state.packet.lock().unwrap();
                                    *shared = Some(p);
                                }
                                None => println!("v2 rejected by magic/version/crc"),
                            },
                            n => println!("Unexpected length: {}", n),
                        }
                    }
                    Ok(None) => {}
                    Err(e) => {
                        println!("Radio RX error: {:?}", e);
                        std::thread::sleep(Duration::from_millis(500));
                    }
                }
            }
        }
        Err(e) => println!("Radio init error: {:?}", e),
    }
}   



async fn handler(State(state): State<AppState>) -> String {
    let packet = state.packet.lock().unwrap();
    match *packet {
        Some(ref p) => format!(
            "Node: {}, Wind: {:.1} m/s ({:.1} kn) — gust {:.1}, lull {:.1}, window {} s / {} samples, Battery: {} mV, Seq: {}, Flags: sensor_ok={}, batt_low={}, window_truncated={}",
            p.node_id,
            (p.wind_avg as f32) / 10.0,
            (p.wind_avg as f32) / 10.0 * 1.94384,
            (p.wind_gust as f32) / 10.0,
            (p.wind_lull as f32) / 10.0,
            p.window_s,
            p.n_sub,
            p.battery_mv,
            p.sequence,
            p.flags & 0x01 != 0,
            p.flags & 0x02 != 0,
            p.flags & 0x04 != 0,
        ),
        None => "Station Starting up".to_string(),
    }
}
