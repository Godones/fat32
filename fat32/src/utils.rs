pub fn u16_from_le_bytes(bytes: &[u8]) -> u16 {
    u16::from_le_bytes(bytes.try_into().unwrap())
}
pub fn u32_from_le_bytes(bytes: &[u8]) -> u32 {
    u32::from_le_bytes(bytes.try_into().unwrap())
}

pub const BLOCK_SIZE: usize = 512;
pub const ENTRY_PER_SECTOR: usize = BLOCK_SIZE / 4;

#[macro_export]
macro_rules! block_buffer {
    () => {
        [0u8; BLOCK_SIZE]
    };
}
