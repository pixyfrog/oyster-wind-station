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



