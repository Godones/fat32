#![feature(associated_type_defaults)]
#![feature(error_in_core)]
#![no_std]

extern crate alloc;

use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::error::Error;
use core::fmt::Debug;

/// 文件夹和普通文件都被视作文件
/// 但是文件夹可以有子文件夹和子文件，而普通文件只能读取/删除/写入数据
pub trait DirectoryLike: Debug + Send + Sync {
    type Error: Debug + Error + 'static;
    fn create_dir(&self, name: &str) -> Result<(), Self::Error>;
    fn create_file(&self, name: &str) -> Result<(), Self::Error>;
    fn delete_dir(&self, name: &str) -> Result<(), Self::Error>;
    fn delete_file(&self, name: &str) -> Result<(), Self::Error>;
    fn cd(&self, name: &str) -> Result<Arc<dyn DirectoryLike<Error = Self::Error>>, Self::Error>;
    fn open(&self, name: &str) -> Result<Arc<dyn FileLike<Error = Self::Error>>, Self::Error>;
    fn list(&self) -> Result<Vec<String>, Self::Error>;
    fn rename_file(&self, old_name: &str, new_name: &str) -> Result<(), Self::Error>;
    fn rename_dir(&self, old_name: &str, new_name: &str) -> Result<(), Self::Error>;
}

pub trait FileLike: Debug + Send + Sync {
    type Error: Debug + Error + 'static;
    fn read(&self, offset: u32, size: u32) -> Result<Vec<u8>, Self::Error>;
    fn write(&self, offset: u32, data: &[u8]) -> Result<u32, Self::Error>;
    fn clear(&self);
    fn size(&self) -> u32;
}
