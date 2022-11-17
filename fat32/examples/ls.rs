use fat32::{BlockDevice, DirectoryLike, Fat32};
use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, Write};
use std::sync::{Arc, Mutex};

fn main() {
    let device = FakeDevice::new("fat32-test/test.img");
    let fat32 = Fat32::new(device).unwrap();
    let root = fat32.root_dir();
    let _ans = root.create_file("test.txt");
    let ans = root.create_dir("test");
    println!("{:?}", ans);
    root.list().unwrap().iter().for_each(|name| {
        println!("{}", name);
    });
    fat32.sync();
}

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
