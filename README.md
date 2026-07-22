# oyster-wind-station
Lora-based wind monitoring for kitesurfing community at Oyster Island, Kampot, Cambodia.

## Architecture
- **Transmitter (Oyster Island):** Raspberry Pi Pico 2 + RFM95W LoRa + anemometer
- **Receiver (Villa Vedici):** Orange Pi 3 LTS + RFM95W LoRa + web dashboard
- **Link:** 10 km LoRa, 868 MHz, custom packet protocol

## Hardware
| Component | Spec | Status |
|-----------|------|--------|
| Pico 2 RP2350 | USB-C, bare-metal Rust | Ordered |
| RFM95W 868 MHz | SX1276, spring antenna | Ordered |
| Anemometer | NPN pulse output, 0-30 m/s | Ordered |
| Orange Pi 3 LTS | Receiver, Armbian 26.5.1 | Ready |
| 18650 Li-ion | Protected, 3500 mAh | Ordered |

## Software
- Transmitter: Rust embedded (`embassy-rs`), deep sleep between readings
- Receiver: Rust (`spidev`, `axum`), web dashboard at `http://<ip>:3000`
- Packet format: 8 bytes — `node_id | wind_speed | battery | sequence | crc`

## Status
- [x] Receiver hardware ready (Orange Pi 3 LTS, SPI enabled)
- [x] Rust toolchain installed
- [ ] LoRa driver implementation
- [ ] Web dashboard
- [ ] Transmitter firmware
- [ ] Field test (10 km link)

## License
MIT
