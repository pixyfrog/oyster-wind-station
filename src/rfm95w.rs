pub const REG_VERSION: u8 = 0x42;
pub const EXPECTED_VERSION: u8 = 0x12;

#[derive(Debug)]
pub enum RadioError {
    InvalidVersion,
    SpiError,
}


pub fn get_version() -> Result<u8, RadioError> {
    let version = read_register_mock(REG_VERSION);
    if version == EXPECTED_VERSION {
        Ok(version)
    } else {
        Err(RadioError::InvalidVersion)
    }
}

pub fn read_register_mock(register: u8) -> u8 {
    if register == REG_VERSION {
        EXPECTED_VERSION
    } else {
        0x00
    }
}

pub struct WindPacket {
    pub node_id: u8,
    pub wind_speed: u16,
    pub battery_mv: u16,
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