#![no_std]
extern crate  alloc;

use alloc::string::{String, ToString};
use alloc::vec::Vec;
use fat32::{Dir, File};

#[repr(C)]
pub struct Tag {
    name: String,
    files: Vec<File>,
    dirs: Vec<Dir>,
}

#[repr(u8)]
pub enum TagType {
    File,
    Dir,
}


#[no_mangle]
pub extern "C" fn rust_function(mut tag:Tag) {
    tag.name = "123213".to_string();
}


#[no_mangle]
pub extern "C" fn rust_function2(ctype:TagType) {

}