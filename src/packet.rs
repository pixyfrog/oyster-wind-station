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

// ---- packet v2 18 bytes ---

pub const MAGIC: u8 = 0xAA;
pub const VERSION_V2: u8 = 0x02;

#[derive(Debug)]
pub struct WindPacketV2 {
    pub node_id: u8,
    pub wind_avg: u16,
    pub wind_gust: u16,
    pub wind_lull: u16,
    pub battery_mv: u16,
    pub sequence: u16,
    pub window_s: u16,
    pub n_sub: u8,
}

pub fn decode_v2(buf: &[u8; 18]) -> Option<WindPacketV2> {
    if buf[0] != MAGIC {
        return None;
    }
    if buf[1] != VERSION_V2 {
        return None;
    }
    if crc8(&buf[0..17]) != buf[17] {
        return None;
    }
    Some(WindPacketV2 {
        node_id: buf[2],
        wind_avg: ((buf[4] as u16) << 8) | (buf[5] as u16),
        wind_gust: ((buf[6] as u16) << 8) | (buf[7] as u16),
        wind_lull: ((buf[8] as u16) << 8) | (buf[9] as u16),
        battery_mv: ((buf[10] as u16) << 8) | (buf[11] as u16),
        sequence: ((buf[12] as u16) << 8) | (buf[13] as u16),
        window_s: ((buf[14] as u16) << 8) | (buf[15] as u16),
        n_sub: buf[16],
    })
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_preserves_all_field() {
        let original = WindPacket {
            node_id : 1,
            wind_speed : 270,
            battery_mv: 3200,
            sequence: 42,
        };
    let encoded = encode(&original);
    let decoded = decode(&encoded).expect("a valid packet must encode");
    assert_eq!(decoded.node_id, original.node_id);
    assert_eq!(decoded.wind_speed, original.wind_speed);
    assert_eq!(decoded.battery_mv, original.battery_mv);
    assert_eq!(decoded.sequence, original.sequence);
    }

    #[test]
    fn reject_bad_magic () {
        let mut buf = encode(&read_packet_mock());
        buf[0] = 0x00;
        buf[8] = crc8(&buf[0..8]); // keep the CRC valid — now ONLY the magic check can catch this
        assert!(decode(&buf).is_none());
    }

    #[test]
    fn reject_corrupted_data_byte () {
        let mut buf = encode(&read_packet_mock());
        buf[3]= buf[3].wrapping_add(1); // corrupt wind speed, CRC now stale
        assert!(decode(&buf).is_none());
    }

    #[test]
    fn reject_bad_crc () {
        let mut buf = encode(&read_packet_mock());
        buf[8] = buf[8].wrapping_add(1); // tamper with the CRC itself
        assert!(decode(&buf).is_none());
    }
    #[test]
    fn decode_v2_reads_the_fields() {
        let mut buf = [0u8; 18];
        buf[0] = 0xAA;
        buf[1] = 0x02;
        buf[2] = 1;
        buf[4] = 0x01; buf[5] = 0x90;    // avg   400  = 40.0 m/s
        buf[6] = 0x02; buf[7] = 0x58;    // gust  600
        buf[8] = 0x00; buf[9] = 0x64;    // lull  100
        buf[10] = 0x0E; buf[11] = 0x74;  // batt  3700 mV
        buf[12] = 0x00; buf[13] = 0x2A;  // seq   42
        buf[14] = 0x02; buf[15] = 0x58;  // window_s 600
        buf[16] = 200;                   // n_sub
        buf[17] = crc8(&buf[0..17]);

        let p = decode_v2(&buf).expect("valid v2 packet must decode");
        assert_eq!(p.wind_avg, 400);
        assert_eq!(p.wind_gust, 600);
        assert_eq!(p.wind_lull, 100);
        assert_eq!(p.battery_mv, 3700);
        assert_eq!(p.sequence, 42);
        assert_eq!(p.window_s, 600);
        assert_eq!(p.n_sub, 200);
}
}
