use crate::cache::{get_block_cache_by_id, CacheManager, CACHE_MANAGER, sync};
use crate::device::{BlockDevice, DEVICE};
use crate::dir::{Dir, DirEntryType, File};
use crate::entry::{EntryFlags, LongEntry, ShortEntry};
use crate::utils::{u16_from_le_bytes, u32_from_le_bytes, BLOCK_SIZE};
use crate::{block_buffer, Content, EntryBytes, Fat, FatEntry, FsInfo, MetaData, BPB};
use alloc::rc::Weak;
use alloc::sync::Arc;
use bitflags::bitflags;
use core::fmt::{Debug, Formatter};
use log::{error, info};
use spin::{Mutex, RwLock};

#[derive(Debug)]
pub struct Fat32 {
    meta_data: Arc<MetaData>,
    fat: Arc<RwLock<Fat>>,
    root_dir: Arc<RwLock<Dir>>,
}

impl Fat32 {
    pub fn new<T: BlockDevice>(device: T) -> Result<Fat32, ()> {
        /// 需要读取第一扇区构建原始信息
        let mut buffer = block_buffer!();
        let dbr = device.read(0, &mut buffer).unwrap();
        /// todo!忽略了正确性检查
        // self.check();
        let meta_data = MetaData {
            bytes_per_sector: u16_from_le_bytes(&buffer[0xb..0xb + 2]),
            sectors_per_cluster: buffer[0xd],
            reserved_sectors: u16_from_le_bytes(&buffer[0xe..0xe + 2]),
            number_of_fats: buffer[0x10],
            total_sectors_32: u32_from_le_bytes(&buffer[0x20..0x20 + 4]),
            sectors_per_fat_32: u32_from_le_bytes(&buffer[0x24..0x24 + 4]),
            root_dir_cluster: u32_from_le_bytes(&buffer[0x2c..0x2c + 4]),
            fs_info_sector: u16_from_le_bytes(&buffer[0x30..0x30 + 2]),
        };
        device
            .read(meta_data.fs_info_sector as usize, &mut buffer)
            .unwrap();
        let fs_info = FsInfo::new(&buffer);
        if !fs_info.is_valid() {
            error!("fs_info is not valid");
            return Err(());
        }
        unsafe {
            CACHE_MANAGER.call_once(|| Box::new(CacheManager::new(100)));
        }

        DEVICE.call_once(|| Arc::new(Mutex::new(device)));
        let fat = Fat::new(Arc::new(meta_data), Arc::new(fs_info));
        fat.print_usage();

        let root_dir = Dir::new(
            meta_data.root_dir_cluster,
            None,
            "/".to_string(),
            EntryFlags::DIRECTORY,
        );
        Ok(Fat32 {
            meta_data: Arc::new(meta_data),
            fat: Arc::new(RwLock::new(fat)),
            root_dir: Arc::new(RwLock::new(root_dir)),
        })
    }
    pub fn list(&self, path: &str) -> Result<Vec<String>, ()> {
        let mut dir = self.root_dir.clone();
        let mut path = path.split("/");
        let mut name = path.next();
        while let Some(n) = name {
            if n == "" {
                name = path.next();
                continue;
            }
            let mut lock = dir.write();
            lock.load(self.meta_data.clone());
            drop(lock);
            let lock = dir.read();
            let sub_dir = lock.cd(n);
            match sub_dir {
                Some(d) => {
                    drop(lock);
                    dir = d;
                    name = path.next();
                }
                None => return Err(()),
            }
        }
        let mut lock = dir.write();
        lock.load(self.meta_data.clone());
        drop(lock);
        let lock = dir.read();
        Ok(lock.list())
    }

    /// 创建一个文件
    /// 必须保证路径中的目录都存在
    pub fn create_file(&self,path:&str)->Result<(),CreateError>{
        if !path.starts_with("/") {
            return Err(CreateError::PathError);
        }
        let mut dir = self.root_dir.clone();
        let mut path = path.split("/").collect::<Vec<&str>>();
        for sub_dir in path.iter().take(path.len()-1){
            if  sub_dir.is_empty(){
                continue;
            }
            let mut lock = dir.write();
            lock.load(self.meta_data.clone());
            drop(lock);
            let lock = dir.read();
            match lock.cd(sub_dir) {
                Some(d) => {
                    drop(lock);
                    dir = d;
                }
                None => return Err(CreateError::DirNotFound),
            }
        }
        let mut lock = dir.write();
        lock.load(self.meta_data.clone());
        drop(lock);
        let mut  lock = dir.write();
        let file_name = path.last().unwrap();
        if lock.find_file(file_name).is_some(){
            return Err(CreateError::FileExist);
        }
        let mut fat = self.fat.write();
        let cluster = fat.alloc_cluster();
        if cluster.is_none(){
            return Err(CreateError::NoSpace);
        }
        let cluster = cluster.unwrap();
        let file = File::new(cluster,file_name.to_string(),0,EntryFlags::ARCHIVE);
        let file = Arc::new(RwLock::new(file));
        lock.add_sub_file(file,self.meta_data.clone(),self.fat.clone());
        Ok(())
    }

    /// 创建一个文件夹
    /// 解析文件夹路径，如果不存在则创建
    pub fn create_dir(&self, path: &str) -> Result<(), ()> {
        if !path.starts_with("/") {
            return Err(());
        }
        let path = path.split("/").collect::<Vec<&str>>();
        let mut dir = self.root_dir.clone();
        for i in 1..path.len() {
            let name = path[i];
            if name.is_empty() {
                return Err(());
            }
            let mut lock = dir.write();
            lock.load(self.meta_data.clone());
            if let Some(entry) = lock.cd(name) {
                drop(lock);
                dir = entry
            } else {
                let cluster = self.fat.write().alloc_cluster();
                if let Some(cluster) = cluster {
                    info!("create dir {} at cluster {:?}", name, cluster);
                    // 分配成功
                    // 写入分配表
                    self.fat.write().set_entry(cluster, FatEntry::Eof,DirEntryType::Dir);
                    let parent = Arc::downgrade(&dir);
                    let sub_dir = Dir::new(
                        cluster,
                        Some(parent),
                        name.to_string(),
                        EntryFlags::DIRECTORY,
                    );
                    let sub_dir = Arc::new(RwLock::new(sub_dir));

                    lock.add_sub_dir(
                        sub_dir.clone(),
                        DirEntryType::Dir,
                        self.meta_data.clone(),
                        self.fat.clone(),
                    );
                    // 创建 . 和 .. 目录
                    // 这两个目录不占用簇
                    let sub_sub_dot =
                        Dir::new(cluster, None, ".".to_string(), EntryFlags::DIRECTORY);
                    let sub_sub_dot = Arc::new(RwLock::new(sub_sub_dot));
                    // .. 目录指向父目录的簇
                    let sub_sub_dot_dot = Dir::new(
                        lock.start_cluster(),
                        None,
                        "..".to_string(),
                        EntryFlags::DIRECTORY,
                    );
                    let sub_sub_dot_dot = Arc::new(RwLock::new(sub_sub_dot_dot));
                    sub_dir.write().add_sub_dir(
                        sub_sub_dot,
                        DirEntryType::Dot,
                        self.meta_data.clone(),
                        self.fat.clone(),
                    );
                    sub_dir.write().add_sub_dir(
                        sub_sub_dot_dot,
                        DirEntryType::DotDot,
                        self.meta_data.clone(),
                        self.fat.clone(),
                    );
                } else {
                    return Err(());
                }
            }
        }
        Ok(())
    }
    pub fn sync(&self){
        sync();
    }
}

bitflags! {
    pub struct OpenFlags: u32 {
        const READ = 0b00000001;
        const WRITE = 0b00000010;
        const APPEND = 0b00000100;
        const TRUNCATE = 0b00001000;
        const CREATE = 0b00010000;
        const EXCLUSIVE = 0b00100000;
    }
}

#[derive(Debug)]
pub enum CreateError {
    PathError,
    /// 目录不存在
    DirNotFound,
    /// 文件已存在
    FileExist,
    /// 磁盘空间不足
    NoSpace,
    /// 磁盘已满
    DiskFull,
    /// 磁盘错误
    DiskError,
}