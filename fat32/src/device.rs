use alloc::sync::Arc;
use spin::once::Once;
use spin::Mutex;

/// the block device should be able to read and write blocks
pub trait BlockDevice: Send + Sync + 'static {
    type Error = ();
    fn read(&self, block: usize, buf: &mut [u8]) -> Result<usize, Self::Error>;
    fn write(&self, block: usize, buf: &[u8]) -> Result<usize, Self::Error>;
    fn flush(&self) -> Result<(), Self::Error>;
}

pub static DEVICE: Once<Arc<Mutex<dyn BlockDevice<Error = ()>>>> = Once::new();
