mod packet;
mod rfm95w;


use axum::{routing::get, Router, extract::State};
use std::time::Duration;
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
                        if bytes.len() == 9 {
                            let mut raw = [0u8; 9];
                            raw.copy_from_slice (&bytes);
                        
                        match packet::decode(&raw) {
                            Some(p) => {
                                println!(
                                    "DECODED node = {}, wind = {:.1}, battery = {}, sequence = {}",
                                    p.node_id,
                                    (p.wind_speed as f32) / 10.0,
                                    p.battery_mv,
                                    p.sequence
                                );
                                let mut shared = state.packet.lock().unwrap();
                                *shared = Some (p);
                            }
                            none => {
                                println!("Packet rejected by magic/crc");
                            }
                        }
                        } else { 
                        println!("Ignoring packet with unexpected lenght");
                        }
                    }
                    Ok(none)=>{}
                    Err(e) => {
                        println!("Radio RX error: {:?}", e);
                        std::thread::sleep(Duration::from_millis(500));
                    }
                }
            }
        }
        Err(e)=> println!("Radio init error: {:?}", e),
    }

}

async fn handler(State(state): State<AppState>) -> String {
    let packet = state.packet.lock().unwrap();
    match *packet {
        Some(ref p) => format!(
            "Node: {}, Wind: {} (m/s), Battery: {} mV, Seq: {}",
            p.node_id, 
            (p.wind_speed as f32) / 10.0, 
            p.battery_mv, 
            p.sequence),
        None => "Station Starting up".to_string(),
    }
}