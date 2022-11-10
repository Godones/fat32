//! 抽象的文件系统目录结构
//!
//! 文件的打开/创建/删除等操作都通过这树个形结构来完成,创建文件系统后处于根目录下
//!

use crate::cache::get_block_cache_by_id;
use crate::entry::{EntryFlags, FullLoongEntry, LongEntry, ShortEntry};
use crate::{Content, EntryBytes, Fat, FatEntry, MetaData, BPB};
use alloc::collections::BTreeMap;
use alloc::sync::Arc;
use alloc::sync::Weak;
use core::cmp::{max, min};
use std::f32::consts::E;
use log::{info, warn};
use spin::RwLock;

#[derive(Debug, Clone)]
pub struct Dir {
    /// 当前目录的起始簇号
    start_cluster: u32,
    /// 当前目录的父目录
    parent: Option<Weak<RwLock<Dir>>>,
    /// 当前目录的名字
    name: String,
    attributes: EntryFlags,
    /// 当前目录的子目录
    sub_directory: BTreeMap<String, Arc<RwLock<Dir>>>,
    /// 当前目录的文件
    files: BTreeMap<String, Arc<RwLock<File>>>,
    /// 是否已经加载
    loaded: bool,
}

#[derive(Debug, Clone)]
pub struct File {
    /// 文件的起始簇号
    start_cluster: u32,
    /// 文件的名字
    name: String,
    /// 文件的大小
    size: u32,
    /// 文件的属性
    attributes: EntryFlags,
}

impl Dir {
    pub fn new(
        start_cluster: u32,
        parent: Option<Weak<RwLock<Dir>>>,
        name: String,
        attr: EntryFlags,
    ) -> Self {
        Dir {
            start_cluster,
            parent,
            name,
            attributes: attr,
            sub_directory: BTreeMap::new(),
            files: BTreeMap::new(),
            loaded: false,
        }
    }
    /// 读取目录的内容
    pub fn load(&mut self, meta: Arc<MetaData>) {
        if self.loaded {
            return;
        }
        let cluster = self.start_cluster;
        let sector = meta.offset_of_cluster(cluster);
        let cache = get_block_cache_by_id(sector);
        cache.read(0, |content: &Content| {
            let mut full_long_entry = FullLoongEntry::new();
            for entry in content.iter::<EntryBytes>() {
                //判断此项是否是合法的
                info!("entry:{:?}",entry);
                if entry[0] == 0x00 {
                    return;
                } else if entry[0] == 0xE5 || entry[0] == 0x05 {
                    // 已经被弃用，但没有删除
                } else {
                    // 根据第11位判断是长文件名还是短文件名
                    let entry_flag = EntryFlags::from_bits(entry[11]).unwrap();
                    info!("entry_flag:{:?}",entry_flag);
                    if entry_flag.contains(EntryFlags::LONG_NAME) {
                        let long_entry = LongEntry::from_buffer(entry);
                        full_long_entry.push(long_entry);
                    } else if entry_flag.is_empty() {
                        return;
                    } else {
                        let short_entry = ShortEntry::from_buffer(entry);
                        // 此时到达一个新的短文件名,需要将之前的长文件名解析出来
                        let mut name = full_long_entry.filename();
                        if name.is_empty() {
                            // 如果长文件名为空，说明是一个. 和 .. 目录
                            name = short_entry.filename();
                        }
                        println!("name:{}",name);
                        full_long_entry.clear();
                        // 判断是文件还是目录
                        if short_entry.attr().contains(EntryFlags::DIRECTORY) {
                            let parent = Arc::new(RwLock::new(self.clone()));
                            let parent = Arc::downgrade(&parent);
                            let dir = Dir::new(
                                short_entry.start_cluster(),
                                Some(parent),
                                name.clone(),
                                short_entry.attr().clone(),
                            );
                            self.sub_directory.insert(name, Arc::new(RwLock::new(dir)));
                        } else {
                            let file = File::new(
                                short_entry.start_cluster(),
                                name.clone(),
                                short_entry.file_size(),
                                short_entry.attr().clone(),
                            );
                            self.files.insert(name, Arc::new(RwLock::new(file)));
                        }
                    }
                }
            }
        });
        self.loaded = true;
    }
    /// 处理目录项名称
    /// 1.当文件名小于8个字符时，不用关心
    /// 2.当文件名大于8个字符，需要查找当前目录下是否有同名的文件，如果有，需要在文件名后面加上数字
    fn name_to_short_name(&self, name: &str) -> String {
        if name=="." || name==".."{
            return name.to_string();
        }
        let mut short_name = String::new();
        let mut name = name.to_string();
        let mut ext = String::new();
        if name.contains(".") {
            let mut split = name.rsplitn(2, ".");
            ext = split.next().unwrap().to_string();
            name = split.next().unwrap().to_string();
        };
        if name.len() > 8 {
            // 查找是否有同名的文件
            let mut i = 1;
            self.sub_directory.iter().for_each(|(key, _)| {
                if key.starts_with(&name) {
                    i += 1;
                }
            });
            self.files.iter().for_each(|(key, _)| {
                if key.starts_with(&name) {
                    i += 1;
                }
            });
            // 计算数字的长度
            // 0-999999
            let num = i.to_string();
            let name = name.chars().take(8 - num.len()).collect::<String>();
            short_name = format!("{}~{}", name, num);
        } else {
            short_name = name;
        }
        if ext.len() != 0 {
            short_name = format!("{}.{}", short_name, ext);
        }
        if short_name.len() > 11 {
            short_name = short_name.chars().take(11).collect::<String>();
        }
        short_name
    }

    fn add_sub_dir_inner(&self,dir:Arc<RwLock<Dir>>,dtypr:DirEntryType,meta:Arc<MetaData>,fat:Arc<RwLock<Fat>>){

    }

    fn make_entry(&self,name:&str,start_cluster:u32,dtype: DirEntryType)->Result<(ShortEntry,FullLoongEntry),()>{
        let short_name = self.name_to_short_name(name);
        warn!("short_name:{}", short_name);
        let attr = match dtype {
            DirEntryType::Dir => EntryFlags::DIRECTORY,
            DirEntryType::File => EntryFlags::ARCHIVE,
            _ => EntryFlags::empty(),
        };

        let short_entry = ShortEntry::new(&short_name, attr, start_cluster);
        // 长目录项
        // 如果是.或者..目录项，不需要长目录项
        let full_long_entry = match dtype {
            DirEntryType::Dot | DirEntryType::DotDot => FullLoongEntry::new(),
            DirEntryType::Dir|DirEntryType::File => FullLoongEntry::from_file_name(&name, short_entry.checksum()),
            _ => FullLoongEntry::new(),
        };
        // assert_eq!(full_long_entry.filename(),short_entry.filename());
        info!("full_long_entry:{:#?}", full_long_entry);
        info!("short_entry:{:#?}", short_entry);
        Ok((short_entry,full_long_entry))
    }

    fn find_enough_space(&self,need_entry_count:usize,meta:Arc<MetaData>,fat:Arc<RwLock<Fat>>)->Result<Vec<(usize,usize)>,()>{
        let mut target_sectors = Vec::new();
        let mut start_sector = meta.offset_of_cluster(self.start_cluster);
        let mut start_cluster = self.start_cluster;
        loop {
            let cache = get_block_cache_by_id(start_sector);
            cache.read(0, |content: &Content| {
                for (index,bytes) in content.iter::<EntryBytes>().enumerate(){
                    if bytes[0] == 0x00 || bytes[0] == 0xE5 {
                        target_sectors.push((start_sector, index));
                        info!("At sector:{},index:{}", start_sector, index);
                        if target_sectors.len() == need_entry_count {
                            break
                        }
                    }
                }
            });
            // 如果已经找到足够的空间，就不需要再继续查找了
            // 否则继续查找下一个扇区
            if target_sectors.len() == need_entry_count {
                break;
            }
            start_sector += 1;
            // 检查是否到达了簇的末尾
            if start_sector == meta.offset_of_cluster(start_cluster + 1) {
                // 如果到达了簇的末尾，就需要分配一个新的簇
                let new_cluster = fat.write().alloc_cluster();
                if new_cluster.is_none() {
                    return Err(());
                }
                //todo!(多线程)
                // 将新的簇添加到当前目录的簇链中
                fat.read()
                    .set_entry(start_cluster, FatEntry::Cluster(new_cluster.unwrap()),DirEntryType::Dir);
                // 修改当前目录的大小
                fat.read().set_entry(new_cluster.unwrap(), FatEntry::Eof,DirEntryType::Dir);
                // 重置扇区号
                start_sector = meta.offset_of_cluster(new_cluster.unwrap());
                // 重置簇号
                start_cluster = new_cluster.unwrap();
            }
            // 检查是否没有可用的簇了
            if start_cluster == meta.free_cluster_count() {
                return Err(());
            }
        }
        Ok(target_sectors)
    }

    fn write_entries(&self,short_entry:&ShortEntry,full_long_entry:&FullLoongEntry,target_sectors:&Vec<(usize,usize)>){
        // / 将长目录项写入到磁盘中
        // 倒序写入
        full_long_entry
            .iter()
            .rev()
            .enumerate()
            .for_each(|(index, entry)| {
                let (sector, offset) = target_sectors[index];
                let cache = get_block_cache_by_id(sector);
                cache.write(offset*32, |content: &mut EntryBytes| {
                    let entry = entry.to_buffer();
                    content.copy_from_slice(&entry);
                });
            });
        // 将短目录项写入到磁盘中
        let (sector, offset) = target_sectors[full_long_entry.len()];
        let cache = get_block_cache_by_id(sector);
        cache.write(offset*32, |content: &mut EntryBytes| {
            let entry = short_entry.to_buffer();
            content.copy_from_slice(&entry);
        });
    }
    /// 在当前目录下创建目录项
    pub fn add_sub_dir(
        &mut self,
        dir: Arc<RwLock<Dir>>,
        dtype: DirEntryType,
        meta: Arc<MetaData>,
        fat: Arc<RwLock<Fat>>,
    ) -> Result<(), ()> {
        let name = dir.read().name.clone();
        let start_cluster = dir.read().start_cluster;
        let (short_entry,full_long_entry) = self.make_entry(name.clone().as_ref(),start_cluster,dtype)?;
        let need_entry_count = full_long_entry.len() + 1;
        let target_sectors = self.find_enough_space(need_entry_count,meta.clone(),fat.clone())?;
        self.write_entries(&short_entry,&full_long_entry,&target_sectors);
        self.sub_directory.insert(name.to_string(), dir);
        Ok(())
    }
    pub fn add_sub_file(&mut self, file: Arc<RwLock<File>>,meta:Arc<MetaData>,fat:Arc<RwLock<Fat>>)->Result<(),()>{
        let name = file.read().name.clone();
        let start_cluster = file.read().start_cluster;
        let (short_entry,full_long_entry) = self.make_entry(name.clone().as_ref(),start_cluster,DirEntryType::File)?;
        let need_entry_count = full_long_entry.len() + 1;
        let target_sectors = self.find_enough_space(need_entry_count,meta.clone(),fat.clone())?;
        self.write_entries(&short_entry,&full_long_entry,&target_sectors);
        self.files.insert(name, file);
        Ok(())
    }
    /// cd 到子目录
    pub fn cd(&self, name: &str) -> Option<Arc<RwLock<Dir>>> {
        if name == "." {
            return Some(Arc::new(RwLock::new(self.clone())));
        }else if name == ".." {
            return match &self.parent {
                Some(parent) => Some(parent.upgrade().unwrap()),
                None => None,
            };
        }
        let file = self.sub_directory.get(name);
        match file {
            Some(dir) => Some(dir.clone()),
            None => None,
        }
    }
    pub fn open(&self, name: &str) -> Option<Arc<RwLock<File>>> {
        let file = self.files.get(name);
        match file {
            Some(file) => Some(file.clone()),
            None => None,
        }
    }
    pub fn list(&self) -> Vec<String> {
        let mut list = Vec::new();
        for dir in self.sub_directory.values() {
            list.push(dir.read().name.clone());
        }
        for file in self.files.values() {
            list.push(file.read().name.clone());
        }
        list
    }
    pub fn find_dir(&self, name: &str) -> Option<Arc<RwLock<Dir>>> {
        self.sub_directory.get(name).map(|dir| dir.clone())
    }
    pub fn find_file(&self, name: &str) -> Option<Arc<RwLock<File>>> {
        self.files.get(name).map(|file| file.clone())
    }
    pub fn start_cluster(&self) -> u32 {
        self.start_cluster
    }
}

impl File {
    pub fn new(start_cluster: u32, name: String, size: u32, attr: EntryFlags) -> Self {
        File {
            start_cluster,
            name,
            size,
            attributes: attr,
        }
    }
    pub fn read(&self, offset: u32, size: u32, meta: Arc<MetaData>) -> Vec<u8> {
        // 偏移量大于文件大小则直接返回空
        if offset >= self.size {
            return Vec::new();
        }
        // 最多只能读取文件的大小
        let mut size = min(size, self.size - offset);
        // 起始扇区号
        let sector = meta.offset_of_cluster(self.start_cluster);
        let mut data = Vec::new();
        data.reserve(size as usize);

        let start_sector = sector + (offset / meta.bytes_per_sector as u32) as usize;
        let end_sector = sector + ((offset + size) / meta.bytes_per_sector as u32) as usize;
        let mut offset = offset;

        for i in start_sector..end_sector {
            let cache = get_block_cache_by_id(i);
            cache.read(0, |content: &Content| {
                let content = content.read();
                let start = (offset % meta.bytes_per_sector as u32) as usize;
                let end = min(start + size as usize, meta.bytes_per_sector as usize);
                data.extend_from_slice(&content[start..end]);
                size -= (end - start) as u32;
                offset += (end - start) as u32;
            });
        }
        data
    }
    pub fn write(&self, offset: u32, data: Vec<u8>) {
        // TODO
    }
    pub fn print_content(&self, meta: Arc<MetaData>) {
        let content = self.read(0, self.size, meta);
        let content = String::from_utf8_lossy(content.as_slice());
        println!("{}", content.as_ref());
    }
}

#[derive(PartialOrd, PartialEq)]
pub enum DirEntryType {
    Dot,
    DotDot,
    File,
    Dir,
}
