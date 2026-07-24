pub struct WindPacket {
    pub node_id: u8,
    pub wind_speed: u16, // in 0.1 m/s units
    pub battery_mv: u16, // millivolts
    pub sequence: u16,
}

pub fn read_packet_mock() -> WindPacket {
    WindPacket {
        node_id: 1,
        wind_speed: 152,    // 15.2 m/s * 10
        battery_mv: 3700,   // 3.7V in millivolts
        sequence: 42,
    }
}

pub fn encode(packet: &WindPacket) -> [u8;9] {
    let mut buf = [0u8;9];
    buf[0] = 0xAA; // magic
    buf[1] = packet.node_id;
    buf[2] = (packet.wind_speed >> 8) as u8; // high byte
    buf[3] = (packet.wind_speed & 0xFF) as u8; // low byte
    buf[4] = (packet.battery_mv >> 8) as u8; // high
    buf[5] = (packet.battery_mv & 0xFF) as u8; //low
    buf[6] = (packet.sequence >> 8) as u8;
    buf[7] = (packet.sequence & 0xFF) as u8;
    buf[8] = crc8(&buf[0..8]);
    buf
}

pub fn crc8(data: &[u8]) -> u8 {
    let mut crc: u8 = 0;
    for byte in data {
       crc = crc.wrapping_add(*byte) 
    }
    crc
}

pub fn decode(buf: &[u8;9]) -> Option<WindPacket> {
    if buf[0]!=0xAA {
        return None;
    }

    let crc = crc8(&buf[0..8]);
    if crc != buf[8] {
        return None;
    }

    Some(WindPacket {
        node_id: buf [1],
        wind_speed: (((buf[2] as u16) << 8) | (buf[3] as u16)),
        battery_mv: (((buf[4] as u16) << 8) | (buf[5] as u16)),
        sequence: (((buf[6] as u16) << 8) | (buf[7] as u16)),
    })

}