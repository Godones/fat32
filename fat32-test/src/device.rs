use fat32::fat::FAT;
use fatfs::{IoBase, SeekFrom};
use mfat32::BlockDevice;
use spin::Once;
use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, Write};
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone)]
pub struct FakeDevice {
    file: Arc<Mutex<File>>,
}

impl FakeDevice {
    pub fn new(name: &str) -> Self {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(name)
            .unwrap();
        Self {
            file: Arc::new(Mutex::new(file)),
        }
    }
}

impl BlockDevice for FakeDevice {
    type Error = ();
    fn read(&self, block: usize, buf: &mut [u8]) -> Result<usize, ()> {
        let mut file = self.file.lock().unwrap();
        file.seek(std::io::SeekFrom::Start(block as u64 * 512))
            .unwrap();
        file.read(buf).unwrap();
        Ok(0)
    }

    fn write(&self, block: usize, buf: &[u8]) -> Result<usize, ()> {
        let mut file = self.file.lock().unwrap();
        file.seek(std::io::SeekFrom::Start(block as u64 * 512))
            .unwrap();
        file.write(buf).unwrap();
        Ok(0)
    }

    fn flush(&self) -> Result<(), ()> {
        Ok(())
    }
}

#[derive(Debug, Copy, Clone)]
pub struct Device;

static DEVICE: Once<Mutex<File>> = Once::new();

impl Device {
    pub fn new(name: &str) -> Self {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(name)
            .unwrap();
        DEVICE.call_once(|| Mutex::new(file));
        Self
    }
}

impl block_device::BlockDevice for Device {
    type Error = ();

    fn read(
        &self,
        buf: &mut [u8],
        address: usize,
        _number_of_blocks: usize,
    ) -> Result<(), Self::Error> {
        let mut file = DEVICE.get().unwrap().lock().unwrap();
        file.seek(std::io::SeekFrom::Start(address as u64)).unwrap();
        file.read(buf).unwrap();
        Ok(())
    }
    fn write(
        &self,
        buf: &[u8],
        address: usize,
        _number_of_blocks: usize,
    ) -> Result<(), Self::Error> {
        let mut file = DEVICE.get().unwrap().lock().unwrap();
        file.seek(std::io::SeekFrom::Start(address as u64)).unwrap();
        file.write(buf).unwrap();
        Ok(())
    }
}

impl IoBase for FakeDevice {
    type Error = ();
}

impl fatfs::Read for FakeDevice {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
        let mut file = self.file.lock().unwrap();
        file.read(buf).unwrap();
        Ok(0)
    }
    fn read_exact(&mut self, buf: &mut [u8]) -> Result<(), Self::Error> {
        let mut file = self.file.lock().unwrap();
        file.read_exact(buf).unwrap();
        Ok(())
    }
}

impl fatfs::Write for FakeDevice {
    fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
        let mut file = self.file.lock().unwrap();
        file.write(buf).unwrap();
        Ok(0)
    }
    fn write_all(&mut self, buf: &[u8]) -> Result<(), Self::Error> {
        let mut file = self.file.lock().unwrap();
        file.write_all(buf).unwrap();
        Ok(())
    }
    fn flush(&mut self) -> Result<(), Self::Error> {
        let mut file = self.file.lock().unwrap();
        file.flush().unwrap();
        Ok(())
    }
}

impl fatfs::Seek for FakeDevice {
    fn seek(&mut self, pos: SeekFrom) -> Result<u64, Self::Error> {
        // let mut file = self.file.lock().unwrap();
        // file.seek(std::io::SeekFrom::Start(pos as u64))
        //     .unwrap();
        Ok(0)
    }
}
