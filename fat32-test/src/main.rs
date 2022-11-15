#![allow(unused)]
mod logging;
mod other_fat32;
mod device;

use mfat32::{Fat32};
use mfat32::{DirectoryLike, FileLike};
use mfat32::DirEntryType::Dir;
use crate::device::FakeDevice;

fn main() {
    logging::init_logger();
    let device = FakeDevice::new("fat32-test/test.img");
    let fat32 = Fat32::new(device).unwrap();
    let root = fat32.root_dir();
    // // let ans = root.create_dir("test_test_test");
    // // println!("{:?}", ans);
    // // root.list().unwrap().iter().for_each(|name| {
    // //     println!("{}", name);
    // // });
    // // let dir = root.cd("test_test_test").unwrap();
    // // dir.list().unwrap().iter().for_each(|name| {
    // //     println!("{}", name);
    // // });
    // // let ans = dir.create_file("test.txt");
    // // println!("{ans:?}");
    // // fat32.sync();
    // //
    // // let file = dir.open("test.txt").unwrap();
    // // file.clear(); // clear file
    // // println!("file size:{}", file.size());
    // // let txt = file.read(0, 100).unwrap();
    // // println!("txt: {}", core::str::from_utf8(txt.as_slice()).unwrap());
    // //
    // // let w = file.write(0, b"hello world");
    // // println!("{:?}", w);
    // // fat32.sync();
    // // let txt = file.read(0, 20).unwrap();
    // // println!("txt: {}", core::str::from_utf8(txt.as_slice()).unwrap());
    // //
    // // dir.delete_file("test.txt").unwrap();
    // // let ans = dir.open("test.txt");
    // // println!("after delete: {:?}", ans);
    // // let ans = dir.create_file("test.txt");
    // // println!("recreate file: {:?}", ans);
    //
    //
    //
    //
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
    // other_fat32::test_first_fat32();
    // other_fat32::test_second_fat32().unwrap();
}


