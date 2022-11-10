use alloc::sync::Arc;
use spin::once::Once;
use spin::Mutex;

/// the block device should be able to read and write blocks
pub trait BlockDevice: Send + Sync + 'static {
    fn read(&self, block: usize, buf: &mut [u8]) -> Result<usize, ()>;
    fn write(&self, block: usize, buf: &[u8]) -> Result<usize, ()>;
    fn flush(&self) -> Result<(), ()>;
}

pub static DEVICE: Once<Arc<Mutex<dyn BlockDevice>>> = Once::new();
