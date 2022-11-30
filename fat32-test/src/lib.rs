#![feature(associated_type_bounds)]
#![feature(test)]
#![allow(unused)]
mod device;
mod logging;
mod other_fat32;
mod test1_create_list_cd;
mod test2_read_write;
mod test3_delete;
mod test4_rename;
mod test5_attr;

use crate::device::FakeDevice;
use fat32_trait::{DirectoryLike, FileLike};
use mfat32::Fat32;

#[test]
fn intergenerational_test() {
    // create your fat32
    logging::init_logger();
    let device = FakeDevice::new("./test.img");
    let fat32 = Fat32::new(device).unwrap();
    let root = fat32.root_dir();
    // get a directory
    // begin test
    test1_create_list_cd::test1_create_list_cd(root.clone());
    test2_read_write::test2_read_write(root.clone());
    test3_delete::test_delete_file_and_dir(root.clone());
    test4_rename::test_rename(root.clone());
    test5_attr::test_attr(root.clone());
}
