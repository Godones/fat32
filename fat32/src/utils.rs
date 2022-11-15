pub fn u16_from_le_bytes(bytes: &[u8]) -> u16 {
    u16::from_le_bytes(bytes.try_into().unwrap())
}
pub fn u32_from_le_bytes(bytes: &[u8]) -> u32 {
    u32::from_le_bytes(bytes.try_into().unwrap())
}

pub const BLOCK_SIZE: usize = 512;
