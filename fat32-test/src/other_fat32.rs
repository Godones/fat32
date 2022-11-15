use crate::device::{Device, FakeDevice};
use fat32::fat::FAT;
use fat32::file::WriteType;
use fat32::volume::Volume;
use fat32::*;
use fatfs::Write;
use fscommon::BufStream;
use std::fs::OpenOptions;

pub fn test_first_fat32() {
    let device = Device::new("fat32-test/test.img");
    let volume = Volume::new(device);
    let mut root = volume.root_dir();
    let a = root.create_file("test_test_test.txt");
    println!("create test.txt {:?}", a);
    // open file
    let mut file = root.open_file("test_test_test.txt").unwrap();
    // write buffer to file
    file.write(&[80; 1234], WriteType::Append).unwrap();
    println!("write over");
    root.delete_file("test_test_test.txt").unwrap();
    println!("delete file");
    let a = root.create_dir("test_test_test");
    println!("create dir {:?}", a);
    let mut dir = root.cd("test_test_test").unwrap();
    let a = dir.create_file("test.txt");
    println!("create test.txt {:?}", a);
    let a = dir.create_dir("sub_dir");
    println!("create sub_dir {:?}", a);
    let a = dir.create_file("rrrrrrrrrrrr.txt");
    println!("create rrrrrrrrrrrr.txt {:?}", a);
    root.delete_dir("test_test_test").unwrap();
}

use std::env;
use std::fs::File;
use std::io::{self, prelude::*};

use fatfs::{FileSystem, FsOptions};

pub fn test_second_fat32() -> io::Result<()> {
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .open("fat32-test/test.img")
        .unwrap();
    let buf_rdr = BufStream::new(file);
    let fs = FileSystem::new(buf_rdr, FsOptions::new())?;
    let root_dir = fs.root_dir();
    root_dir.create_dir("test_test_test")?;
    root_dir.create_file("test.txt")?;
    root_dir.iter().for_each(|name| {
        if name.is_ok() {
            let t = name.unwrap();
            let x = t.file_name();
            println!("{x}");
        }
    });
    let mut file = root_dir.open_file("test.txt")?;
    let mut buf = vec![];
    file.read_to_end(&mut buf)?;
    print!("{}", String::from_utf8_lossy(&buf));
    Ok(())
}
