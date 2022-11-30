use fat32::{BlockDevice, Fat32};
use fat32_trait::DirectoryLike;
use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, Write};
use std::sync::{Arc, Mutex};

fn main() {
    let device = FakeDevice::new("fat32-test/test.img");
    let fat32 = Fat32::new(device).unwrap();
    let root = fat32.root_dir();
    let ans = root.create_file("test.txt");
    println!("{ans:?}");
    let file = root.open("test.txt").unwrap();
    file.clear(); // clear file
    println!("file size:{}", file.size());
    let txt = file.read(0, 100).unwrap();
    println!("txt: {}", core::str::from_utf8(txt.as_slice()).unwrap());
    let w = file.write(0, b"hello world");
    println!("{:?}", w);
    let txt = file.read(0, 20).unwrap();
    println!("txt: {}", core::str::from_utf8(txt.as_slice()).unwrap());
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
