#![feature(exclusive_range_pattern)]
#![feature(associated_type_defaults)]
#![no_std]
mod cache;
mod device;
mod dir;
mod entry;
mod fat32;
mod layout;
mod utils;

extern crate alloc;

pub use device::BlockDevice;
pub use dir::{Dir, DirectoryLike, File, FileLike, OperationError};
pub use crate::fat32::Fat32;
