mod logging;
use fat32::{BlockDevice, Fat32};
use fat32::{DirectoryLike, FileLike};
use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, Write};
use std::sync::Mutex;
use fat32::DirEntryType::Dir;

fn main() {
    logging::init_logger();
    let device = FakeDevice::new();
    let fat32 = Fat32::new(device).unwrap();
    let root = fat32.root_dir();
    // let ans = root.create_dir("test_test_test");
    // println!("{:?}", ans);
    // root.list().unwrap().iter().for_each(|name| {
    //     println!("{}", name);
    // });
    // let dir = root.cd("test_test_test").unwrap();
    // dir.list().unwrap().iter().for_each(|name| {
    //     println!("{}", name);
    // });
    // let ans = dir.create_file("test.txt");
    // println!("{ans:?}");
    // fat32.sync();
    //
    // let file = dir.open("test.txt").unwrap();
    // file.clear(); // clear file
    // println!("file size:{}", file.size());
    // let txt = file.read(0, 100).unwrap();
    // println!("txt: {}", core::str::from_utf8(txt.as_slice()).unwrap());
    //
    // let w = file.write(0, b"hello world");
    // println!("{:?}", w);
    // fat32.sync();
    // let txt = file.read(0, 20).unwrap();
    // println!("txt: {}", core::str::from_utf8(txt.as_slice()).unwrap());
    //
    // dir.delete_file("test.txt").unwrap();
    // let ans = dir.open("test.txt");
    // println!("after delete: {:?}", ans);
    // let ans = dir.create_file("test.txt");
    // println!("recreate file: {:?}", ans);




    /// test delete file and dir
    let a = root.create_dir("dir");
    println!("create dir {:?}", a);
    let test_dir = root.cd("dir").unwrap();
    let a = test_dir.create_file("test_file.txt");
    println!("create test_file.txt {:?}", a);
    let a = test_dir.create_dir("sub_dir");
    println!("create sub_dir {:?}", a);
    let list = test_dir.list().unwrap();
    for name in list {
        println!("{}", name);
    }
    root.delete_dir("dir").unwrap();
    let a = root.create_file("test.txt");
    println!("create test.txt {:?}", a);
    fat32.sync();
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
