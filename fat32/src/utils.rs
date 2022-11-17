pub fn u16_from_le_bytes(bytes: &[u8]) -> u16 {
    u16::from_le_bytes(bytes[0..2].try_into().unwrap())
}
pub fn u32_from_le_bytes(bytes: &[u8]) -> u32 {
    u32::from_le_bytes(bytes[0..4].try_into().unwrap())
}

pub const BLOCK_SIZE: usize = 512;

#[cfg(test)]
mod tests {
    #[test]
    fn test_u16_from_le_bytes() {
        let bytes = [0x01, 0x02];
        assert_eq!(0x0201, super::u16_from_le_bytes(&bytes));
        let bytes = [0x01, 0x02, 0x03];
        assert_eq!(0x0201, super::u16_from_le_bytes(&bytes));
    }
    #[test]
    #[should_panic]
    fn test_u16_from_le_bytes_length_1() {
        let byte = [0x01];
        assert_eq!(0x01, super::u16_from_le_bytes(&byte));
    }
    #[test]
    fn test_u32_from_le_bytes() {
        let bytes = [0x01, 0x02, 0x03, 0x04];
        assert_eq!(0x04030201, super::u32_from_le_bytes(&bytes));
        let bytes = [0x01, 0x02, 0x03, 0x04, 0x05];
        assert_eq!(0x04030201, super::u32_from_le_bytes(&bytes));
    }
    #[test]
    #[should_panic]
    fn test_u32_from_le_bytes_length_2() {
        let bytes = [0x01, 0x02, 0x03];
        super::u32_from_le_bytes(&bytes);
    }
}
