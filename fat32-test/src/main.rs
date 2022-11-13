mod logging;
use fat32::{BlockDevice, Fat32,};
use fat32::{DirectoryLike, FileLike};
use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, Write};
use std::sync::Mutex;
fn main() {
    logging::init_logger();
    let device = FakeDevice::new();
    let fat32 = Fat32::new(device).unwrap();
    let root = fat32.root_dir();
    let ans = root.create_dir("test");
    println!("{:?}", ans);
    root.list().unwrap().iter().for_each(|name| {
        println!("{}", name);
    });
    let dir = root.cd("test").unwrap();
    dir.list().unwrap().iter().for_each(|name| {
        println!("{}", name);
    });
    let ans = dir.create_file("test.txt");
    println!("{ans:?}");
    fat32.sync();
    let file = dir.open("test.txt").unwrap();
    println!("file size:{}", file.size());
    let txt = file.read(0, 100).unwrap();
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
