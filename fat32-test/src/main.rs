mod logging;

use fat32::{BlockDevice, Fat32};
use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, Write};
use std::mem::size_of;
use std::sync::Mutex;
fn main() {
    logging::init_logger();
    let device = FakeDevice::new();
    let fat32 = Fat32::new(device).unwrap();
    println!("{:#x?}", fat32);
    fat32
        .list("/")
        .unwrap()
        .iter()
        .for_each(|x| println!("{}", x));
    fat32.create_dir("/test1").unwrap();
    // fat32.create_file("/123.txt").unwrap();
    fat32.sync();
    fat32
        .list("/test1")
        .unwrap()
        .iter()
        .for_each(|x| println!("{}", x));

    let txt = fat32.load_binary_data("/hello.txt").unwrap();
    println!("{}", core::str::from_utf8(txt.as_slice()).unwrap());
}

#[derive(Debug)]
struct FakeDevice {
    file: Mutex<File>,
}

impl FakeDevice {
    pub fn new() -> Self {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .open("fat32-test/test.img")
            .unwrap();
        Self {
            file: Mutex::new(file),
        }
    }
}

impl BlockDevice for FakeDevice {
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
