//! 抽象的文件系统目录结构
//!
//! 文件的打开/创建/删除等操作都通过这树个形结构来完成,创建文件系统后处于根目录下
//!
use crate::cache::{get_block_cache_by_id, BlockCache};
use crate::entry::{EntryFlags, FullLoongEntry, LongEntry, ShortEntry};
use crate::{Content, EntryBytes, Fat, FatEntry, MetaData, BPB};
use alloc::collections::BTreeMap;
use alloc::sync::Arc;
use core::cmp::{max, min};
use log::{info, trace, warn};
use spin::{Mutex, RwLock, RwLockWriteGuard};
use std::ops::Range;
use thiserror::Error;

#[derive(Debug, Clone)]
pub struct Dir {
    /// 当前目录的起始簇号
    start_cluster: u32,
    /// 元数据信息
    meta: MetaData,
    /// fat表
    fat: Arc<RwLock<Fat>>,
    sub_dirs: Arc<RwLock<BTreeMap<String, Dir>>>,
    files: Arc<RwLock<BTreeMap<String, File>>>,
}

#[derive(Debug, Clone)]
pub struct File {
    size: u32,
    /// 文件的起始簇号
    start_cluster: u32,
    /// 元数据
    meta: MetaData,
    fat: Arc<RwLock<Fat>>,
}

impl Dir {
    /// 根目录可以使用new进行创建
    /// 其它目录都会由empty进行创建，empty不会读取读取目录下的内容，然后再load目录下的文件
    pub fn new(start_cluster: u32, meta: MetaData, fat: Arc<RwLock<Fat>>) -> Self {
        let dir = Self::empty(start_cluster, meta, fat);
        dir.load();
        dir
    }
    pub fn empty(start_cluster: u32, meta: MetaData, fat: Arc<RwLock<Fat>>) -> Self {
        Dir {
            start_cluster,
            meta,
            fat,
            sub_dirs: Arc::new(Default::default()),
            files: Arc::new(Default::default()),
        }
    }
    /// 从磁盘中读取目录信息
    /// 读取的目录信息包括子目录和文件
    /// 在读取时不能访问子目录和文件，以免出现错误
    fn load(&self) {
        // 当前目录包含的所有扇区号
        let mut sectors = self.clusters_to_sectors();
        let mut sub_dir = self.sub_dirs.write();
        let mut files = self.files.write();
        'outer: loop {
            if sectors.is_empty() {
                break;
            }
            let mut range = sectors.remove(0);
            let mut flag = false;
            for i in range {
                let cache = get_block_cache_by_id(i);
                cache.read(0, |content: &Content| {
                    let mut full_long_entry = FullLoongEntry::new();
                    for entry in content.iter::<EntryBytes>() {
                        //判断此项是否是合法的
                        info!("entry: {:x?}", entry);
                        if entry[0] == 0x00 {
                            flag = true;
                            return;
                        } else if entry[0] == 0xE5 || entry[0] == 0x05 {
                            // 已经被弃用，但没有删除
                        } else {
                            // 根据第11位判断是长文件名还是短文件名
                            let entry_flag = EntryFlags::from_bits(entry[11]).unwrap();
                            info!("entry_flag:{:?}", entry_flag);
                            if entry_flag.contains(EntryFlags::LONG_NAME) {
                                let long_entry = LongEntry::from_buffer(entry);
                                info!("loong checksum: {}",long_entry.check_sum());
                                full_long_entry.push(long_entry);
                            } else {
                                let short_entry = ShortEntry::from_buffer(entry);
                                // 此时到达一个新的短文件名,需要将之前的长文件名解析出来
                                let mut name = full_long_entry.filename();
                                if name.is_empty() {
                                    name = short_entry.filename();
                                } // .和..没有长目录项
                                info!("name:{}", name);
                                full_long_entry.clear();
                                // 判断是否是目录
                                if entry_flag.contains(EntryFlags::DIRECTORY) {
                                    let start_cluster = short_entry.start_cluster();
                                    let dir = Dir::empty(
                                        start_cluster,
                                        self.meta.clone(),
                                        self.fat.clone(),
                                    );
                                    sub_dir.insert(name, dir);
                                } else {
                                    let start_cluster = short_entry.start_cluster();
                                    let size = short_entry.file_size();
                                    info!("checksum: {}",short_entry.checksum());
                                    let file = File::new(
                                        start_cluster,
                                        size,
                                        self.meta.clone(),
                                        self.fat.clone(),
                                    );
                                    files.insert(name, file);
                                }
                            }
                        }
                    }
                }); // read one sector over
                if flag {
                    break 'outer;
                }
            } // read one cluster over
        } // read all cluster over
    }
    /// 处理目录项名称
    /// 1.当文件名小于8个字符时，不用关心
    /// 2.当文件名大于8个字符，需要查找当前目录下是否有同名的文件，如果有，需要在文件名后面加上数字
    fn name_to_short_name(&self, name: &str) -> String {
        if name == "." || name == ".." {
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
            self.sub_dirs.read().iter().for_each(|(key, _)| {
                if key.starts_with(&name) {
                    i += 1;
                }
            });
            self.files.read().iter().for_each(|(key, _)| {
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

    fn make_entry(
        &self,
        name: &str,
        start_cluster: u32,
        dtype: DirEntryType,
    ) -> Result<(ShortEntry, FullLoongEntry), OperationError> {
        let short_name = self.name_to_short_name(name);
        info!("name {name}'s short_name is {}", &short_name);
        let attr = match dtype {
            DirEntryType::Dir | DirEntryType::Dot | DirEntryType::DotDot => EntryFlags::DIRECTORY,
            DirEntryType::File => EntryFlags::ARCHIVE,
            _ => EntryFlags::empty(),
        };

        let short_entry = ShortEntry::new(&short_name, attr, start_cluster);
        // 长目录项
        // 如果是.或者..目录项，不需要长目录项
        let full_long_entry = match dtype {
            DirEntryType::Dot | DirEntryType::DotDot => FullLoongEntry::new(),
            DirEntryType::Dir | DirEntryType::File => {
                FullLoongEntry::from_file_name(&name, short_entry.checksum())
            }
            _ => FullLoongEntry::new(),
        };
        info!("full_long_entry:{:#?}", full_long_entry);
        info!("short_entry:{:#?}", short_entry);
        Ok((short_entry, full_long_entry))
    }

    fn find_enough_space(
        &self,
        need_entry_count: usize,
    ) -> Result<Vec<(usize, usize)>, OperationError> {
        let mut target_sectors = Vec::new();
        let mut start_sector = self.meta.cluster_to_sector(self.start_cluster);
        let mut start_cluster = self.start_cluster;
        loop {
            let cache = get_block_cache_by_id(start_sector);
            cache.read(0, |content: &Content| {
                for (index, bytes) in content.iter::<EntryBytes>().enumerate() {
                    if bytes[0] == 0x00 || bytes[0] == 0xE5 {
                        target_sectors.push((start_sector, index));
                        trace!("alloc entry in sector:{}, offset:{}", start_sector, index);
                        if target_sectors.len() == need_entry_count {
                            break;
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
            if start_sector == self.meta.cluster_to_sector(start_cluster + 1) {
                // 如果到达了簇的末尾，就需要分配一个新的簇
                let new_cluster = self
                    .fat
                    .write()
                    .alloc_cluster()
                    .map_or(Err(OperationError::NoEnoughSpace), |cluster| Ok(cluster))?;
                //todo!(多线程)
                // 将新的簇添加到当前目录的簇链中
                self.fat.read().set_entry(
                    start_cluster,
                    FatEntry::Cluster(new_cluster),
                    DirEntryType::Dir,
                );
                self.fat
                    .read()
                    .set_entry(new_cluster, FatEntry::Eof, DirEntryType::Dir);
                // 重置扇区号
                start_sector = self.meta.cluster_to_sector(new_cluster);
                // 重置簇号
                start_cluster = new_cluster;
            }
            // 检查是否没有可用的簇了
            if start_cluster == self.meta.free_cluster_count() {
                return Err(OperationError::NoEnoughSpace);
            }
        }
        Ok(target_sectors)
    }

    fn write_entries(
        &self,
        short_entry: &ShortEntry,
        full_long_entry: &FullLoongEntry,
        target_sectors: &Vec<(usize, usize)>,
    ) {
        // / 将长目录项写入到磁盘中
        // 倒序写入
        full_long_entry
            .iter()
            .rev()
            .enumerate()
            .for_each(|(index, entry)| {
                let (sector, offset) = target_sectors[index];
                let cache = get_block_cache_by_id(sector);
                cache.write(offset * 32, |content: &mut EntryBytes| {
                    let entry = entry.to_buffer();
                    content.copy_from_slice(&entry);
                });
            });
        // 将短目录项写入到磁盘中
        let (sector, offset) = target_sectors[full_long_entry.len()];
        let cache = get_block_cache_by_id(sector);
        cache.write(offset * 32, |content: &mut EntryBytes| {
            let entry = short_entry.to_buffer();
            content.copy_from_slice(&entry);
        });
    }
    /// 在当前目录下查找指定名称的目录项
    fn find_dir(&self, name: &str) -> Option<Arc<RwLock<Dir>>> {
        todo!()
    }
    fn find_file(&self, name: &str) -> Option<Arc<RwLock<File>>> {
        todo!()
    }
    pub fn start_cluster(&self) -> u32 {
        self.start_cluster
    }
    fn clusters_to_sectors(&self) -> Vec<Range<usize>> {
        // 获取文件夹占用的簇
        let clusters = self.fat.read().get_entry_chain(self.start_cluster);
        let mut ans = Vec::new();
        clusters.iter().for_each(|cluster| {
            let first_sector = self.meta.cluster_to_sector(*cluster);
            let end_sector = first_sector + self.meta.sectors_per_cluster as usize;
            ans.push(first_sector..end_sector);
        });
        ans
    }
    /// 创建.和..目录项
    /// 这两个目录项不占用磁盘空间
    fn add_dir_or_file(
        &self,
        name: &str,
        cluster: u32,
        dtype: DirEntryType,
    ) -> Result<(), OperationError> {
        // 创建目录项
        let (short_entry, full_long_entry) = self.make_entry(name, cluster, dtype)?;
        // 计算目录项占据的空间
        let need_entry_count = full_long_entry.len() + 1;
        // 计算目录项所在的扇区以及偏移
        let target_sectors = self.find_enough_space(need_entry_count)?;
        // 写入目录项
        self.write_entries(&short_entry, &full_long_entry, &target_sectors);
        //
        Ok(())
    }
}

impl DirectoryLike for Dir {
    type Error = OperationError;
    fn create_dir(&self, name: &str) -> Result<(), OperationError> {
        // 创建文件夹时，防止其它线程读取
        let mut sub_dirs = self.sub_dirs.write();
        // 检查是否已经存在同名的文件夹
        if sub_dirs.contains_key(name) {
            return Err(OperationError::DirExist);
        }
        let cluster = self
            .fat
            .write()
            .alloc_cluster()
            .map_or(Err(OperationError::NoEnoughSpace), |cluster| Ok(cluster))?; // 分配簇
        self.fat
            .write()
            .set_entry(cluster, FatEntry::Eof, DirEntryType::Dir); //写入fat表
        info!("create dir {name} at {cluster} cluster");
        self.add_dir_or_file(name, cluster, DirEntryType::Dir);
        // 创建目录
        let dir = Dir::new(cluster, self.meta.clone(), self.fat.clone());
        // 创建目录的.和..目录项
        dir.add_dir_or_file(".", cluster, DirEntryType::Dot);
        dir.add_dir_or_file("..", self.start_cluster, DirEntryType::DotDot);
        dir.sub_dirs.write().insert(".".to_string(), dir.clone());
        dir.sub_dirs.write().insert("..".to_string(), self.clone());
        sub_dirs.insert(name.to_string(), dir);
        Ok(())
    }

    fn create_file(&self, name: &str) -> Result<(), OperationError> {
        let mut sub_files = self.files.write();
        // 检查是否已经存在同名的文件
        if sub_files.contains_key(name) {
            return Err(OperationError::FileExist);
        }
        let cluster = self
            .fat
            .write()
            .alloc_cluster()
            .map_or(Err(OperationError::NoEnoughSpace), |cluster| Ok(cluster))?; // 分配簇
        self.fat
            .write()
            .set_entry(cluster, FatEntry::Eof, DirEntryType::Dir); //写入fat表
        self.add_dir_or_file(name, cluster, DirEntryType::File); //写入目录项
        let file = File::new(cluster, 0, self.meta.clone(), self.fat.clone());
        sub_files.insert(name.to_string(), file); //添加到文件列表
        Ok(())
    }

    fn delete_dir(&self, name: &str) -> Result<(), OperationError> {
        todo!()
    }

    fn delete_file(&self, name: &str) -> Result<(), OperationError> {
        todo!()
    }

    /// 进入子目录
    /// will cd to dir2
    fn cd(&self, path: &str) -> Result<Arc<Dir>, OperationError> {
        let dir = self
            .sub_dirs
            .read()
            .get(path)
            .map_or(Err(OperationError::DirNotFound), |dir| {
                Ok(Arc::new(dir.clone()))
            })?;
        dir.load();
        Ok(dir)
    }

    fn open(&self, name: &str) -> Result<Arc<File>, OperationError> {
        self.files
            .read()
            .get(name)
            .map_or(Err(OperationError::FileNotFound), |file| {
                Ok(Arc::new(file.clone()))
            })
    }

    fn list(&self) -> Result<Vec<String>, OperationError> {
        let mut ans = Vec::new();
        self.sub_dirs.read().iter().for_each(|(name, _)| {
            ans.push(name.clone());
        });
        self.files.read().iter().for_each(|(name, _)| {
            ans.push(name.clone());
        });
        Ok(ans)
    }
}

impl File {
    pub fn new(start_cluster: u32, size: u32, meta: MetaData, fat: Arc<RwLock<Fat>>) -> Self {
        Self {
            size,
            start_cluster,
            meta,
            fat,
        }
    }
    /// 获取文件占用的簇
    /// cluster:[sector]-[sector]-[sector]-[sector]
    ///  |
    /// cluster
    /// |
    /// cluster
    fn clusters_to_sectors(&self, offset: u32, size: u32) -> Vec<usize> {
        // 文件所占的簇链
        let cluster_chain = self.fat.read().get_entry_chain(self.start_cluster);
        // 计算簇内偏移量
        let cluster_offset = offset % self.meta.bytes_per_cluster();
        // 计算开始簇号
        let mut start_cluster_index = offset / self.meta.bytes_per_cluster();
        // 计算簇内的开始扇区号
        let mut start_sector_index = cluster_offset / self.meta.bytes_per_sector as u32;
        let mut ans = Vec::new();
        let mut size = size;
        for cluster in cluster_chain.iter().skip(start_cluster_index as usize) {
            let start_sector = self.meta.cluster_to_sector(*cluster) + start_sector_index as usize;
            let end_sector =
                self.meta.cluster_to_sector(*cluster) + self.meta.sectors_per_cluster as usize;
            for sector in start_sector..end_sector {
                ans.push(sector);
                size -= self.meta.bytes_per_sector as u32;
                if size <= 0 {
                    return ans;
                }
            }
            start_sector_index = 0;
        }
        ans
    }
}

impl FileLike for File {
    type Error = OperationError;

    fn size(&self) -> u32 {
        self.size
    }

    fn read(&self, offset: u32, size: u32) -> Result<Vec<u8>, Self::Error> {
        // 偏移量大于文件大小则直接返回空
        if offset >= self.size {
            return Ok(Vec::new());
        }
        // 最多只能读取文件的大小
        info!("file size is {}", self.size);
        let mut size = min(size, self.size - offset);
        info!("read file at offset:{}, size:{}", offset, size);
        let mut data = Vec::new();
        data.reserve(size as usize);

        let sectors = self.clusters_to_sectors(offset, size);
        let mut offset = offset;

        for i in sectors {
            let cache = get_block_cache_by_id(i);
            cache.read(0, |content: &Content| {
                let content = content.read();
                let start = (offset % self.meta.bytes_per_sector as u32) as usize;
                let end = min(start + size as usize, self.meta.bytes_per_sector as usize);
                data.extend_from_slice(&content[start..end]);
                size -= (end - start) as u32;
                offset += (end - start) as u32;
                if size == 0 {
                    return;
                }
            });
        }
        Ok(data)
    }

    fn write(&self, offset: u32, data: &[u8]) -> Result<u32, Self::Error> {
        todo!()
    }

    fn clear(&self) -> Result<(), Self::Error> {
        todo!()
    }
}

#[derive(PartialOrd, PartialEq)]
pub enum DirEntryType {
    Dot,
    DotDot,
    File,
    Dir,
}

/// 文件夹和普通文件都被视作文件
/// 但是文件夹可以有子文件夹和子文件，而普通文件只能读取/删除/写入数据
pub trait DirectoryLike {
    type Error;
    fn create_dir(&self, name: &str) -> Result<(), Self::Error>;
    fn create_file(&self, name: &str) -> Result<(), Self::Error>;
    fn delete_dir(&self, name: &str) -> Result<(), Self::Error>;
    fn delete_file(&self, name: &str) -> Result<(), Self::Error>;
    fn cd(&self, name: &str) -> Result<Arc<Dir>, Self::Error>;
    fn open(&self, name: &str) -> Result<Arc<File>, Self::Error>;
    fn list(&self) -> Result<Vec<String>, Self::Error>;
}

pub trait FileLike {
    type Error;
    fn size(&self) -> u32;
    fn read(&self, offset: u32, size: u32) -> Result<Vec<u8>, Self::Error>;
    fn write(&self, offset: u32, data: &[u8]) -> Result<u32, Self::Error>;
    fn clear(&self) -> Result<(), Self::Error>;
}

#[derive(Error, Debug)]
pub enum OperationError {
    #[error("No enough space")]
    NoEnoughSpace,
    #[error("File not found")]
    PathError(String),
    #[error("File not found")]
    FileNotFound,
    #[error("Fire Exist")]
    FileExist,
    #[error("Dir Exist")]
    DirExist,
    #[error("Dir not found")]
    DirNotFound,
    #[error("Offset out of size")]
    OffsetOutOfSize,
}
