#![allow(unused)]
mod device;
mod logging;
mod other_fat32;
mod test1_create_list_cd;

use crate::device::FakeDevice;
use mfat32::Fat32;
use mfat32::{DirectoryLike, FileLike};

fn main() {
    logging::init_logger();
    let device = FakeDevice::new("fat32-test/test.img");
    let fat32 = Fat32::new(device).unwrap();
    let root = fat32.root_dir();

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

    let a = root.rename_file("test.txt", "newtest.txt");
    println!("rename test.txt to newtest.txt {:?}", a);

    let a = root.create_dir("test_dir");
    println!("create test_dir {:?}", a);
    let a = root.rename_dir("test_dir", "new_test_dir");
    println!("rename test_dir to new_test_dir {:?}", a);

    fat32.sync();
    // other_fat32::test_first_fat32();
    // other_fat32::test_second_fat32().unwrap();
}
