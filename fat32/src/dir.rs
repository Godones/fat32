//! 抽象的文件系统目录结构
//!
//! 文件的打开/创建/删除等操作都通过这树个形结构来完成,创建文件系统后处于根目录下
//!
use crate::cache::get_block_cache_by_id;
use crate::entry::{EntryFlags, FullLoongEntry, LongEntry, ShortEntry};
use crate::layout::{Bpb, Content, EntryBytes, Fat, FatEntry, MetaData, SectorData};
use crate::utils::u32_from_le_bytes;

use alloc::collections::BTreeMap;
use alloc::format;
use alloc::string::{String, ToString};
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::cmp::{max, min};
use core::ops::Range;
use log::{info, trace};
use spin::RwLock;

#[derive(Debug, Clone)]
pub struct Dir {
    /// 当前目录的起始簇号
    start_cluster: u32,
    /// 目录项位置(sector, offset)
    address: (usize, usize),
    /// 元数据信息
    meta: Arc<MetaData>,
    /// fat表
    fat: Arc<RwLock<Fat>>,
    sub_dirs: Arc<RwLock<BTreeMap<String, Dir>>>,
    files: Arc<RwLock<BTreeMap<String, File>>>,
}

#[derive(Debug, Clone)]
pub struct File {
    /// 文件的起始簇号
    start_cluster: u32,
    /// 元数据
    meta: Arc<MetaData>,
    fat: Arc<RwLock<Fat>>,
    /// 记录文件目录项的位置(sector, offset)
    /// offset是短目录项的位置
    address: (usize, usize),
}

impl Dir {
    /// 根目录可以使用new进行创建
    /// 其它目录都会由empty进行创建，empty不会读取读取目录下的内容，然后再load目录下的文件
    pub fn new(
        start_cluster: u32,
        address: (usize, usize),
        meta: Arc<MetaData>,
        fat: Arc<RwLock<Fat>>,
    ) -> Self {
        let dir = Self::empty(start_cluster, address, meta, fat);
        dir.load();
        dir
    }
    fn empty(
        start_cluster: u32,
        address: (usize, usize),
        meta: Arc<MetaData>,
        fat: Arc<RwLock<Fat>>,
    ) -> Self {
        Dir {
            start_cluster,
            address,
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
            let range = sectors.remove(0);
            let mut flag = false;
            for i in range {
                let cache = get_block_cache_by_id(i);
                cache.read(0, |content: &Content| {
                    let mut full_long_entry = FullLoongEntry::new();
                    for (index, entry) in content.iter::<EntryBytes>().enumerate() {
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
                                        (i, index * 32),
                                        self.meta.clone(),
                                        self.fat.clone(),
                                    );
                                    sub_dir.insert(name, dir);
                                } else {
                                    let start_cluster = short_entry.start_cluster();
                                    info!("checksum: {}", short_entry.check_sum());
                                    let file = File::new(
                                        start_cluster,
                                        (i, index * 32),
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
    fn name_to_short_name(&self, name: &str, dtype: DirEntryType) -> String {
        if name == "." || name == ".." {
            return name.to_string();
        }
        let mut short_name;
        let mut name = name.to_string();
        let mut ext = String::new();
        if name.contains('.') {
            let mut split = name.rsplitn(2, '.');
            ext = split.next().unwrap().to_string();
            name = split.next().unwrap().to_string();
        };
        if name.len() > 8 {
            // 查找是否有同名的文件
            let mut i = 1;
            match dtype {
                DirEntryType::Dir => {
                    let sub_dirs = self.sub_dirs.read();
                    sub_dirs.iter().for_each(|(key, _)| {
                        if key.starts_with(&name) {
                            i += 1;
                        }
                    });
                }
                DirEntryType::File => {
                    let files = self.files.read();
                    files.iter().for_each(|(key, _)| {
                        if key.starts_with(&name) {
                            i += 1;
                        }
                    });
                }
                _ => {}
            }
            // 计算数字的长度
            // 0-999999
            let num = i.to_string();
            let name = name.chars().take(8 - num.len() - 1).collect::<String>();
            short_name = format!("{name}~{num}");
        } else {
            short_name = name;
        }
        if !ext.is_empty() {
            short_name = format!("{short_name}.{ext}");
        }
        if short_name.len() > 12 {
            short_name = short_name.chars().take(12).collect::<String>();
        }
        short_name
    }

    fn make_entry(
        &self,
        name: &str,
        short_name: &str,
        start_cluster: u32,
        dtype: DirEntryType,
    ) -> Result<(ShortEntry, FullLoongEntry), OperationError> {
        info!("name {name}'s short_name is {}", short_name);
        let attr = match dtype {
            DirEntryType::Dir | DirEntryType::Dot | DirEntryType::DotDot => EntryFlags::DIRECTORY,
            DirEntryType::File => EntryFlags::ARCHIVE,
        };

        let short_entry = ShortEntry::new(short_name, attr, start_cluster);
        // 长目录项
        // 如果是.或者..目录项，不需要长目录项
        let full_long_entry = match dtype {
            DirEntryType::Dot | DirEntryType::DotDot => FullLoongEntry::new(),
            DirEntryType::Dir | DirEntryType::File => {
                FullLoongEntry::from_file_name(name, short_entry.check_sum())
            }
        };
        info!("full_long_entry:{:#?}", full_long_entry);
        info!("short_entry:{:#?}", short_entry);
        Ok((short_entry, full_long_entry))
    }

    fn find_enough_entry_inner(
        &self,
        sector: usize,
        index: usize,
        j: usize,
        need: usize,
        collect: &mut Vec<(usize, usize, usize)>,
    ) {
        let cache = get_block_cache_by_id(sector);
        cache.read(0, |content: &Content| {
            let content = content.read();
            for i in 0..content.len() / 32 {
                let entry_bytes = &content[i * 32..(i + 1) * 32];
                if entry_bytes[0] == 0x00 || entry_bytes[0] == 0xE5 {
                    // 找到空闲目录项，判断是否是与之前找到的目录项连续
                    if collect.is_empty() {
                        collect.push((index, j, i));
                    } else {
                        let (last_index, last_j, last_i) = collect.last().unwrap();
                        if (index == *last_index && j == *last_j && i == *last_i + 1)
                            || (index == *last_index && j == *last_j + 1 && i == 0)
                            || (index == *last_index + 1 && j == 0 && i == 0)
                        {
                            collect.push((index, j, i));
                        } else {
                            collect.clear();
                            collect.push((index, j, i));
                        }
                    }
                }
                if collect.len() == need {
                    return;
                } // 找到足够的目录项,退出查找
            } // one sectors
        });
    }

    /// # 找到足够的位置存放目录项
    /// 这些位置必须是连续的,由于长目录项最大为8个，段目录项为1个,
    /// 9个目录项位置只会存在一个sector或者连续的两个sector中,
    /// 但这两个sector可能会在不同的cluster中
    fn find_enough_entry(&self, need: usize) -> Result<Vec<(usize, usize)>, OperationError> {
        info!("find_enough_entry need:{}", need);
        let mut fat = self.fat.write();
        let cluster = self.start_cluster;
        let mut cluster_chain = fat.get_cluster_chain(cluster); // 获取簇链

        let mut collect = Vec::new();
        collect.reserve(need); // 预分配空间
        let mut find_flag = false;

        trace!("begin to find entries");
        for (index, &cluster) in cluster_chain.iter().enumerate() {
            let start_sector = self.meta.cluster_to_sector(cluster);
            let end_sector = start_sector + self.meta.sectors_per_cluster as usize;
            for (j, sector) in (start_sector..end_sector).into_iter().enumerate() {
                self.find_enough_entry_inner(sector, index, j, need, &mut collect); //在一个sector中查找
                if collect.len() == need {
                    find_flag = true;
                    break;
                }
            } // all sectors
            if find_flag {
                break;
            }
        } // all cluster
          // 检查是否找到足够的目录项
        if !find_flag {
            // 没有找到足够的目录项，需要分配新的cluster
            let new_cluster = fat
                .alloc_cluster()
                .map_or(Err(OperationError::NoEnoughSpace), |cluster| Ok(cluster))?;
            let cluster = cluster_chain.last().unwrap();
            fat.set_entry(*cluster, FatEntry::Cluster(new_cluster), DirEntryType::Dir);
            fat.set_entry(new_cluster, FatEntry::Eof, DirEntryType::Dir);
            // 重新查找
            // 此时保证了新分配的cluster一定是可以满足分配
            // 但不需要重新开始分配
            cluster_chain.push(new_cluster);
            let new_sector = self.meta.cluster_to_sector(new_cluster);
            self.find_enough_entry_inner(new_sector, cluster_chain.len(), 0, need, &mut collect);
            assert_eq!(collect.len(), need);
        }
        // 找到足够的目录项，返回结果
        let ans = collect
            .iter()
            .map(|(cluster_index, sector_index, offset)| {
                let cluster = cluster_chain[*cluster_index];
                let sector = self.meta.cluster_to_sector(cluster) + *sector_index;
                (sector, *offset)
            })
            .collect::<Vec<(usize, usize)>>();
        info!("find_enough_entry ans:{:#?}", ans);
        Ok(ans)
    }

    fn write_entries(
        &self,
        short_entry: &ShortEntry,
        full_long_entry: &FullLoongEntry,
        target_sectors: &[(usize, usize)],
    ) -> Result<(usize, usize), OperationError> {
        // 将长目录项写入到磁盘中
        // 倒序写入
        trace!("write long entries....");
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
        trace!("write short entry....");
        let (sector, offset) = target_sectors[full_long_entry.len()];
        let cache = get_block_cache_by_id(sector);
        cache.write(offset * 32, |content: &mut EntryBytes| {
            let entry = short_entry.to_buffer();
            content.copy_from_slice(&entry);
        });
        trace!("write entry success....");
        Ok((sector, offset * 32))
    }
    fn clusters_to_sectors(&self) -> Vec<Range<usize>> {
        // 获取文件夹占用的簇
        let clusters = self.fat.read().get_cluster_chain(self.start_cluster);
        let mut ans = Vec::new();
        clusters.iter().for_each(|cluster| {
            let first_sector = self.meta.cluster_to_sector(*cluster);
            let end_sector = first_sector + self.meta.sectors_per_cluster as usize;
            ans.push(first_sector..end_sector);
        });
        ans
    }
    fn add_dir_or_file(
        &self,
        name: &str,
        short_name: &str,
        cluster: u32,
        dtype: DirEntryType,
    ) -> Result<(usize, usize), OperationError> {
        info!(
            "add_dir_or_file name:{},cluster:{},dtype:{:?}",
            name, cluster, dtype
        );
        // 创建目录项
        let (short_entry, full_long_entry) = self.make_entry(name, short_name, cluster, dtype)?;
        // 计算目录项占据的空间
        let need_entry_count = full_long_entry.len() + 1;
        // 计算目录项所在的扇区以及偏移
        let target_sectors = self.find_enough_entry(need_entry_count)?;
        // 写入目录项
        let addr = self.write_entries(&short_entry, &full_long_entry, &target_sectors)?;
        Ok(addr)
    }

    /// 删除目录项
    fn delete_entry(
        &self,
        start_cluster: u32,
        address: (usize, usize),
        cluster_chain: &[u32],
    ) -> Result<(), OperationError> {
        let ans = cluster_chain.iter().enumerate().find(|(_i, &cluster)| {
            let s_sector = self.meta.cluster_to_sector(cluster);
            let e_sector = s_sector + self.meta.sectors_per_cluster as usize;
            (s_sector..e_sector).contains(&address.0)
        });
        assert!(ans.is_some()); //
        let (index, _) = ans.unwrap();
        // 处理目录项跨扇区或者跨簇的情况
        let cache = get_block_cache_by_id(address.0);
        trace!("delete short entry at {}, offset {}", address.0, address.1);
        let short_entry = cache.write(address.1, |entry: &mut EntryBytes| {
            let short_entry = ShortEntry::from_buffer(entry);
            entry[0] = 0xE5;
            short_entry
        });
        assert_eq!(short_entry.start_cluster(), start_cluster);
        // 处理长目录项,需要逆向查找其位置
        // 只要找到段目录项所在扇区以及其前一个扇区(可能位于前一个簇内)
        let pre_sector = if address.0 != self.meta.cluster_to_sector(cluster_chain[index]) {
            // 如果不是簇的第一个扇区，则前一个扇区就是当前扇区的前一个扇区
            address.0 - 1
        } else {
            // 如果是簇的第一个扇区，则前一个扇区可能可能不存在
            if index > 0 {
                self.meta.cluster_to_sector(cluster_chain[index - 1])
                    + self.meta.sectors_per_cluster as usize
                    - 1
            } else {
                // 前一个扇区不存在的情况，说明目录项一定全部位于当前扇区
                address.0
            }
        };
        let mut entry_offset = address.1;
        let mut entry_sector = address.0;
        let mut flag = false;
        loop {
            let (sector, offset) = if entry_offset == 0 {
                // 如果是扇区的第一个目录项，则需要查找前一个扇区的最后一个目录项
                if pre_sector == entry_sector {
                    assert_eq!(pre_sector, self.meta.root_dir_start_sector());
                    trace!("stop find long entry");
                    break;
                } //
                assert_ne!(pre_sector, entry_sector);
                let t = (pre_sector, self.meta.bytes_per_sector as usize - 32);
                entry_sector = pre_sector;
                entry_offset = self.meta.bytes_per_sector as usize - 32;
                t
            } else {
                let t = (entry_sector, entry_offset - 32);
                entry_offset -= 32;
                t
            };
            trace!("find long entry in sector {}, offset {}", sector, offset);
            let cache = get_block_cache_by_id(sector);
            cache.write(offset, |entry_bytes: &mut EntryBytes| {
                let entry_attr = EntryFlags::from_bits(entry_bytes[11]).unwrap();
                trace!("entry attr: {:?}", entry_attr);
                if entry_attr.contains(EntryFlags::LONG_NAME) {
                    // 如果是长目录项，则将其标记为删除
                    let entry = LongEntry::from_buffer(entry_bytes);
                    assert_eq!(entry.check_sum(), short_entry.check_sum());
                    entry_bytes[0] = 0xE5;
                    trace!("delete long entry");
                } else {
                    // 如果是段目录项，则直接返回
                    flag = true;
                    trace!("stop find long entry");
                }
            });
            if flag {
                break;
            }
        }
        Ok(())
    }

    /// 清空目录下的所有文件和目录
    fn clear(&self) -> Result<(), OperationError> {
        if self.sub_dirs.read().is_empty() && self.files.read().is_empty() {
            return Ok(());
        }
        let file_names = self.files.read().keys().cloned().collect::<Vec<String>>();
        file_names.iter().for_each(|file| {
            self.delete_file(file).unwrap();
        });
        let dir_names = self
            .sub_dirs
            .read()
            .keys()
            .cloned()
            .collect::<Vec<String>>();
        dir_names.iter().for_each(|dir| {
            self.delete_dir(dir).unwrap();
        });
        // 回收簇
        let fat = self.fat.write();
        let cluster_chain = fat.get_cluster_chain(self.start_cluster);
        trace!("clear dir, cluster_chain: {:?}", cluster_chain);
        for &i in cluster_chain.iter().skip(1) {
            fat.set_entry(i, FatEntry::Free, DirEntryType::File);
        } // 跳过了第一个簇
          // 将第一个簇指向结束标志
        fat.set_entry(self.start_cluster, FatEntry::Eof, DirEntryType::File);
        Ok(())
    }
}

impl DirectoryLike for Dir {
    type Error = OperationError;
    fn create_dir(&self, name: &str) -> Result<(), OperationError> {
        let short_name = self.name_to_short_name(name, DirEntryType::Dir);
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
        let address = self.add_dir_or_file(name, &short_name, cluster, DirEntryType::Dir)?;
        // 创建目录
        let dir = Dir::empty(cluster, address, self.meta.clone(), self.fat.clone());
        // 创建目录的.和..目录项
        dir.add_dir_or_file(".", ".", cluster, DirEntryType::Dot)?;
        dir.add_dir_or_file("..", "..", self.start_cluster, DirEntryType::DotDot)?;
        dir.sub_dirs.write().insert(".".to_string(), dir.clone());
        dir.sub_dirs.write().insert("..".to_string(), self.clone());
        sub_dirs.insert(name.to_string(), dir);
        Ok(())
    }

    fn create_file(&self, name: &str) -> Result<(), OperationError> {
        let short_name = self.name_to_short_name(name, DirEntryType::File);
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
        let address = self.add_dir_or_file(name, &short_name, cluster, DirEntryType::File)?; //写入目录项
        let file = File::new(cluster, address, self.meta.clone(), self.fat.clone());
        sub_files.insert(name.to_string(), file); //添加到文件列表
        Ok(())
    }

    /// 删除文件夹
    /// 需要递归删除
    fn delete_dir(&self, name: &str) -> Result<(), OperationError> {
        trace!("delete dir: {}", name);
        if name == "." || name == ".." {
            return Ok(());
        }
        let mut sub_dirs = self.sub_dirs.write();
        let dir = sub_dirs.remove(name).ok_or(OperationError::DirNotFound)?;
        dir.clear()?;
        // 递归删除子文件夹
        let start_cluster = dir.start_cluster;
        // 删除分配的簇
        self.fat
            .write()
            .set_entry(start_cluster, FatEntry::Free, DirEntryType::Dir);
        // 删除目录项
        info!("begin to delete dir entry...");
        let cluster_chain = self.fat.read().get_cluster_chain(self.start_cluster);
        self.delete_entry(dir.start_cluster, dir.address, &cluster_chain)?;
        info!("delete dir entry success");
        Ok(())
    }

    /// 删除文件
    /// 清空文件内容，并将fat表中的簇释放
    /// 删除目录项
    fn delete_file(&self, name: &str) -> Result<(), OperationError> {
        trace!("delete file {}", name);
        let mut sub_file = self.files.write();
        // 检查是否存在此文件
        let file = sub_file.remove(name).ok_or(OperationError::FileNotFound)?;
        // 清空文件内容
        trace!("clear file content");
        file.clear();
        // 释放簇
        trace!("free cluster");
        let fat = self.fat.write();
        fat.set_entry(file.start_cluster, FatEntry::Free, DirEntryType::File);
        // 删除目录项
        // File 包含了文件的短目录项位置,需要找到长目录项的位置
        let cluster_chain = fat.get_cluster_chain(self.start_cluster); //获取目录的簇链
        self.delete_entry(file.start_cluster, file.address, &cluster_chain)?; //删除目录项
        Ok(())
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

    /// 返回当前目录下的文件与子目录
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
    /// 重命名某个文件
    fn rename_file(&self, old_name: &str, new_name: &str) -> Result<(), Self::Error> {
        let file = self
            .files
            .write()
            .remove(old_name)
            .ok_or(OperationError::FileNotFound)?;
        let short_name = self.name_to_short_name(new_name, DirEntryType::File);
        let mut files = self.files.write();
        // 删除原来的目录项
        let cluster_chain = self.fat.read().get_cluster_chain(self.start_cluster); //获取目录的簇链
        self.delete_entry(file.start_cluster, file.address, &cluster_chain)?; //删除目录项
                                                                              // 生成新的目录项
        self.add_dir_or_file(
            new_name,
            &short_name,
            file.start_cluster,
            DirEntryType::File,
        )?;
        files.insert(new_name.to_string(), file);
        Ok(())
    }
    /// 重命名某个目录
    fn rename_dir(&self, old_name: &str, new_name: &str) -> Result<(), Self::Error> {
        let dir = self
            .sub_dirs
            .write()
            .remove(old_name)
            .ok_or(OperationError::DirNotFound)?;
        let short_name = self.name_to_short_name(new_name, DirEntryType::Dir);
        // 删除原来的目录项
        let cluster_chain = self.fat.read().get_cluster_chain(self.start_cluster); //获取目录的簇链
        self.delete_entry(dir.start_cluster, dir.address, &cluster_chain)?; //删除目录项
                                                                            // 生成新的目录项
        self.add_dir_or_file(new_name, &short_name, dir.start_cluster, DirEntryType::Dir)?;
        self.sub_dirs.write().insert(new_name.to_string(), dir);
        Ok(())
    }
}

impl File {
    pub fn new(
        start_cluster: u32,
        address: (usize, usize),
        meta: Arc<MetaData>,
        fat: Arc<RwLock<Fat>>,
    ) -> Self {
        Self {
            start_cluster,
            meta,
            fat,
            address,
        }
    }
    fn empty()->Self{
        Self{
            start_cluster: 0,
            meta: Arc::new(Default::default()),
            fat: Arc::new(RwLock::new(Fat::empty())),
            address: (0, 0)
        }
    }
    /// 获取文件占用的簇
    /// cluster:[sector]-[sector]-[sector]-[sector]
    ///  |
    /// cluster
    /// |
    /// cluster
    fn calculate_sectors_without_alloc(
        &self,
        offset: u32,
        size: u32,
        cluster_chain: &[u32],
    ) -> Vec<usize> {
        // 计算簇内偏移量
        let cluster_offset = offset % self.meta.bytes_per_cluster();
        // 计算开始簇号
        let start_cluster_index = offset / self.meta.bytes_per_cluster();
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
                size = size.saturating_sub(self.meta.bytes_per_sector as u32);
                if size == 0 {
                    return ans;
                }
            }
            start_sector_index = 0;
        }
        ans
    }
    /// 计算在offset处写入size个字节需要当前文件增加的簇数
    /// 并且计算新文件大小
    fn calculate_addition_cluster(&self, offset: u32, w_size: u32) -> (usize, u32) {
        let mut need_cluster = (offset + w_size) / self.meta.bytes_per_cluster();
        if (offset + w_size) % self.meta.bytes_per_cluster() != 0 {
            need_cluster += 1;
        }
        // 计算文件已经占用的簇数
        let size = self.size();
        let mut used_cluster = size / self.meta.bytes_per_cluster();
        if size % self.meta.bytes_per_cluster() != 0 || size == 0 {
            used_cluster += 1;
        }
        // 计算需要增加的簇数
        let need_cluster = need_cluster.saturating_sub(used_cluster);
        let new_size = max(size, offset + w_size);
        (need_cluster as usize, new_size)
    }
    pub fn size(&self) -> u32 {
        let cache = get_block_cache_by_id(self.address.0);
        info!("file at :({},{})", self.address.0, self.address.1);
        let mut size = 0;
        cache.read(0, |content: &Content| {
            let content = content.read();
            size = u32_from_le_bytes(&content[self.address.1 + 28..self.address.1 + 32]);
        });
        size
    }
    fn update_size(&self, size: u32) {
        let cache = get_block_cache_by_id(self.address.0);
        cache.write(0, |content: &mut Content| {
            let content = content.write();
            let size = size.to_le_bytes();
            content[self.address.1 + 28..self.address.1 + 32].copy_from_slice(&size);
        });
    }
}

impl FileLike for File {
    type Error = OperationError;

    fn read(&self, offset: u32, _size: u32) -> Result<Vec<u8>, Self::Error> {
        // 偏移量大于文件大小则直接返回空
        let size = self.size();
        if offset >= size {
            return Ok(Vec::new());
        }
        // 最多只能读取文件的大小
        let mut size = min(size, size - offset);
        info!("read file at offset:{}, size:{}", offset, size);
        let mut data = Vec::new();
        // 提前分配空间
        data.reserve(size as usize);

        // 拿到fat的读锁，防止其它线程修改fat表
        let fat = self.fat.read();
        // 文件所占的簇链
        let cluster_chain = fat.get_cluster_chain(self.start_cluster);
        // 计算需要读取的扇区
        let sectors = self.calculate_sectors_without_alloc(offset, size, &cluster_chain);
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

    /// 向文件中写入数据
    /// 1. 如果文件大小不够则分配簇
    /// 2. 如果文件大小够则直接写入
    fn write(&self, offset: u32, data: &[u8]) -> Result<u32, Self::Error> {
        // 拿到fat的写锁，防止其它线程修改fat表
        let mut fat = self.fat.write();
        // 计算额外需要的簇数
        let (addition, new_size) = self.calculate_addition_cluster(offset, data.len() as u32);
        info!("addition :{}, new_size :{}", addition, new_size);
        info!("file_start_cluster: {}", self.start_cluster);
        // 计算文件已经分配的簇
        let mut old_cluster_chain = fat.get_cluster_chain(self.start_cluster);
        info!("old_cluster_chain :{:?}", old_cluster_chain);
        // 开始分配额外的簇
        let mut begin = *old_cluster_chain.last().unwrap(); // 原文件的最后一个簇
        for _ in 0..addition {
            let cluster = fat
                .alloc_cluster()
                .map_or(Err(OperationError::NoEnoughSpace), |cluster| Ok(cluster))?; // 分配簇
            old_cluster_chain.push(cluster); // 将新分配的簇加入队列
            fat.set_entry(begin, FatEntry::Cluster(cluster), DirEntryType::File); // 将原文件的最后一个簇指向新分配的簇
            begin = cluster; // 更新原文件的最后一个簇
        }
        // 最后一个簇指向结束标志
        fat.set_entry(begin, FatEntry::Eof, DirEntryType::File);
        info!("new_cluster_chain :{:?}", old_cluster_chain);

        // 找到offset位于的扇区位置
        let sectors =
            self.calculate_sectors_without_alloc(offset, data.len() as u32, &old_cluster_chain);
        let mut offset = offset;
        let mut size = data.len() as u32;
        let mut data_start = 0;
        for i in sectors {
            let cache = get_block_cache_by_id(i);
            cache.write(0, |content: &mut SectorData| {
                let start = (offset % self.meta.bytes_per_sector as u32) as usize;
                let end = min(start + size as usize, self.meta.bytes_per_sector as usize);
                info!("write: {start}-{end} in sector {i}");
                content[start..end].copy_from_slice(&data[data_start..data_start + (end - start)]);
                size -= (end - start) as u32;
                offset += (end - start) as u32;
                data_start += end - start;
            });
            if size == 0 {
                break;
            }
        }
        // 更新文件大小 todo!()
        self.update_size(new_size);
        Ok(data.len() as u32)
    }

    /// 清空文件内容
    /// 释放所有簇
    fn clear(&self) {
        trace!("clear file");
        let fat = self.fat.write();
        let cluster_chain = fat.get_cluster_chain(self.start_cluster);
        trace!("clear file, cluster_chain: {:?}", cluster_chain);
        for &i in cluster_chain.iter().skip(1) {
            fat.set_entry(i, FatEntry::Free, DirEntryType::File);
        } // 跳过了第一个簇
          // 将第一个簇指向结束标志
        fat.set_entry(self.start_cluster, FatEntry::Eof, DirEntryType::File);
        // 更新文件大小
        self.update_size(0);
    }
}

#[derive(PartialOrd, PartialEq, Debug)]
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
    fn rename_file(&self, old_name: &str, new_name: &str) -> Result<(), Self::Error>;
    fn rename_dir(&self, old_name: &str, new_name: &str) -> Result<(), Self::Error>;
}

pub trait FileLike {
    type Error;
    fn read(&self, offset: u32, size: u32) -> Result<Vec<u8>, Self::Error>;
    fn write(&self, offset: u32, data: &[u8]) -> Result<u32, Self::Error>;
    fn clear(&self);
}

#[derive(Debug)]
pub enum OperationError {
    NoEnoughSpace,
    FileNotFound,
    FileExist,
    DirExist,
    DirNotFound,
    OffsetOutOfSize,
    InvalidDirName,
    NotFound,
}

#[cfg(test)]
mod tests{
    use super::*;
    fn make_dir()->Dir{
        Dir{
            start_cluster: 0,
            address: (0, 0),
            meta: Arc::new(MetaData::default()),
            fat: Arc::new(RwLock::new(Fat::empty())),
            sub_dirs: Arc::new(Default::default()),
            files: Arc::new(Default::default())
        }
    }
    #[test]
    fn test_name_to_short_name(){
        let dir = make_dir();
        let name1 = "hello1234.txt";
        let short_name = dir.name_to_short_name(name1,DirEntryType::File);
        assert_eq!("hello1~1.txt",short_name);
        dir.files.write().insert("hello1234.txt".to_string(),File::empty());
        let short_name = dir.name_to_short_name(name1,DirEntryType::File);
        assert_eq!("hello1~2.txt",short_name);
        let short_name = dir.name_to_short_name(name1,DirEntryType::Dir);
        assert_eq!("hello1~1.txt",short_name);
        dir.sub_dirs.write().insert(name1.to_string(),dir.clone());
        let short_name = dir.name_to_short_name(name1,DirEntryType::Dir);
        assert_eq!("hello1~2.txt",short_name);
    }
    #[test]
    fn test_make_entry(){
        let dir = make_dir();
        let name1 = "hello1234.txt";
        let short_name = dir.name_to_short_name(name1,DirEntryType::File);
        let (short_entry,full_long_entry) = dir.make_entry(name1,&short_name,0,DirEntryType::File).unwrap();
        assert_eq!(short_entry.start_cluster(),0);
        assert_eq!(short_entry.filename(),short_name.to_uppercase());
        assert_eq!(short_entry.attr(),&EntryFlags::ARCHIVE);
        assert_eq!(full_long_entry.filename(),name1);
        assert_eq!(full_long_entry.len(),1);
        full_long_entry.iter().for_each(|long_entry|{
            assert_eq!(long_entry.check_sum(),short_entry.check_sum());
        })
    }
}