use crate::cache::{get_block_cache_by_id};
use crate::dir::DirEntryType;
use crate::utils::u32_from_le_bytes;
use crate::utils::{BLOCK_SIZE};
use core::fmt::{Debug};
use alloc::sync::Arc;

pub type EntryBytes = [u8; 32];
pub type SectorData = [u8; BLOCK_SIZE];

/// 只包含部分需要的BPB参数
#[derive(Debug, Copy, Clone)]
pub struct MetaData {
    pub bytes_per_sector: u16,
    pub sectors_per_cluster: u8,
    pub reserved_sectors: u16,
    pub number_of_fats: u8,
    pub total_sectors_32: u32,
    pub sectors_per_fat_32: u32,
    pub root_dir_cluster: u32,
    pub fs_info_sector: u16,
}

/// 从BiosParameterBlock需要提供的功能
pub trait BPB {
    /// 拿到fat1/fat2的起始扇区
    fn fat_start_sector(&self) -> usize;
    /// 拿到根目录的起始扇区
    fn root_dir_start_sector(&self) -> usize;
    /// 根据数据区的簇号得到数据区的起始扇区
    fn cluster_to_sector(&self, cluster: u32) -> usize;
    fn sectors_per_fat_32(&self) -> usize;
    fn free_cluster_count(&self) -> u32;
    fn bytes_per_cluster(&self) -> u32;
}

impl BPB for MetaData {
    #[inline]
    fn fat_start_sector(&self) -> usize {
        self.reserved_sectors as usize
    }
    #[inline]
    fn root_dir_start_sector(&self) -> usize {
        self.fat_start_sector() + self.number_of_fats as usize * self.sectors_per_fat_32 as usize
    }
    #[inline]
    fn cluster_to_sector(&self, cluster: u32) -> usize {
        self.root_dir_start_sector() + (cluster - 2) as usize * self.sectors_per_cluster as usize
    }
    #[inline]
    fn sectors_per_fat_32(&self) -> usize {
        self.sectors_per_fat_32 as usize
    }
    #[inline]
    fn free_cluster_count(&self) -> u32 {
        //计算空闲簇数
        (self.total_sectors_32 - self.root_dir_start_sector() as u32)
            / self.sectors_per_cluster as u32
            - 2
    }

    fn bytes_per_cluster(&self) -> u32 {
        self.bytes_per_sector as u32 * self.sectors_per_cluster as u32
    }
}

/// File Allocation Table
#[derive(Debug)]
pub struct Fat {
    meta_data: Arc<MetaData>,
    next_free_cluster: u32,
    total_free_cluster: u32,
}
/// 文件分配表的表项
#[derive(Debug)]
pub enum FatEntry {
    /// 0x00000000
    Free,
    /// 0xFFFFFFF7
    Bad,
    /// 0x0FFFFFFF
    Eof,
    Cluster(u32),
}

impl Fat {
    pub fn new(meta_data: Arc<MetaData>, fs_info: Arc<FsInfo>) -> Self {
        Self {
            meta_data,
            next_free_cluster: fs_info.next_free_cluster,
            total_free_cluster: fs_info.free_cluster_count,
        }
    }
    pub fn get_entry(&self, cluster: u32) -> FatEntry {
        let fat_sector = self.meta_data.fat_start_sector() + (cluster as usize * 4) / BLOCK_SIZE;
        let fat_offset = (cluster as usize * 4) % BLOCK_SIZE;
        let sector_cache = get_block_cache_by_id(fat_sector);
        let entry = sector_cache.read(fat_offset, |val: &u32| *val);
        match entry {
            0x00000000 => FatEntry::Free,
            0xFFFFFFF7 => FatEntry::Bad,
            0x0FFFFFF8..=0x0FFFFFFF => FatEntry::Eof,
            _ => FatEntry::Cluster(entry),
        }
    }
    pub fn set_entry(&self, cluster: u32, entry: FatEntry, dirtype: DirEntryType) {
        let fat_sector = self.meta_data.fat_start_sector() + (cluster as usize * 4) / BLOCK_SIZE;
        let fat_offset = (cluster as usize * 4) % BLOCK_SIZE;
        let mut sector_cache = get_block_cache_by_id(fat_sector);
        let entry = match entry {
            FatEntry::Free => [0, 0, 0, 0],
            FatEntry::Bad => [0xF7, 0xFF, 0xFF, 0xFF],
            FatEntry::Eof => {
                if dirtype == DirEntryType::Dir {
                    [0xF8, 0xFF, 0xFF, 0x0F]
                } else {
                    [0xFF, 0xFF, 0xFF, 0x0F]
                }
            }
            FatEntry::Cluster(entry) => entry.to_le_bytes(),
        };
        sector_cache.write(fat_offset, |val: &mut u32| {
            *val = u32_from_le_bytes(&entry);
        });
    }

    pub fn alloc_cluster(&mut self) -> Option<u32> {
        if self.total_free_cluster == 0 {
            return None;
        }
        let mut cluster = self.next_free_cluster;
        let mut entry = self.get_entry(cluster);
        loop {
            match entry {
                FatEntry::Free => {
                    self.total_free_cluster -= 1;
                    self.next_free_cluster = cluster + 1;
                    return Some(cluster);
                }
                _ => {
                    cluster += 1;
                    if cluster >= self.meta_data.total_sectors_32 as u32 {
                        cluster = 2;
                    }
                    entry = self.get_entry(cluster);
                }
            }
        }
    }

    pub fn get_cluster_chain(&self, cluster: u32) -> Vec<u32> {
        let mut chain = Vec::new();
        let mut cluster = cluster;
        let mut entry = self.get_entry(cluster);
        loop {
            match entry {
                FatEntry::Eof => {
                    chain.push(cluster);
                    return chain;
                }
                FatEntry::Cluster(next) => {
                    chain.push(cluster);
                    entry = self.get_entry(next);
                    cluster = next;
                }
                _ => {
                    panic!("bad cluster chain :{:?}", entry);
                }
            }
        }
    }

    pub fn print_usage(&self) {
        let start = self.meta_data.fat_start_sector();
        let end = start + self.meta_data.sectors_per_fat_32() as usize;
        'outer: for i in start..end {
            let sector_cache = get_block_cache_by_id(i);
            let mut flag = false;
            sector_cache.read(0, |content: &Content| {
                for val in content.iter::<u32>() {
                    if *val == 0 {
                        flag = true;
                        break;
                    }
                    println!("{:#x?}", *val);
                }
            });
            if flag {
                break 'outer;
            }
        }
    }
}

/// 通常fs_info保存在第一个扇区中
#[derive(Debug)]
pub struct FsInfo {
    /// 0x41615252
    /// 这个值被用来标识这个扇区是fs_info
    pub lead_signature: u32,
    /// 0x61417272
    /// 表明该扇区已经被使用
    pub struct_signature: u32,
    /// 0xffffffff
    /// 保存最新的剩余簇数量，如果为 0xFFFFFFFF 表示剩余簇未
    /// 知，需要重新计算，初此之外其他的值都可以用，而且不要
    /// 求十分精确，但必须保证其值<=磁盘所有的簇数。
    pub free_cluster_count: u32,
    /// 通常这个值被设定为驱动程序最后分配出去的
    /// 簇号。如果值为 0xFFFFFFFF，那么驱动程序必须从簇 2 开
    /// 始查找，除此之外其他的值都可以使用，当然前提是这个值
    /// 必须合法的
    pub next_free_cluster: u32,
    /// 0xaa550000
    pub trail_signature: u32,
}

impl FsInfo {
    /// 从磁盘中读取fs_info
    pub fn new(data: &[u8]) -> Self {
        Self {
            lead_signature: u32_from_le_bytes(&data[0..4]),
            struct_signature: u32_from_le_bytes(&data[484..488]),
            free_cluster_count: u32_from_le_bytes(&data[488..492]),
            next_free_cluster: u32_from_le_bytes(&data[492..496]),
            trail_signature: u32_from_le_bytes(&data[508..512]),
        }
    }
    #[inline]
    pub fn is_valid(&self) -> bool {
        self.lead_signature == 0x41615252
            && self.struct_signature == 0x61417272
            && self.trail_signature == 0xaa550000
    }
}

/// 从缓存读取一个扇区的内容
pub struct Content {
    data: [u8; BLOCK_SIZE],
}

impl Content {
    pub fn new() -> Self {
        Self {
            data: [0; BLOCK_SIZE],
        }
    }
    pub fn iter<T>(&self) -> impl Iterator<Item = &T> {
        self.data
            .chunks_exact(std::mem::size_of::<T>())
            .map(|x| unsafe { &*(x.as_ptr() as *const T) })
    }
    pub fn read(&self) -> &[u8] {
        &self.data
    }
    pub fn write(&mut self) -> &mut [u8] {
        &mut self.data
    }
}

#[derive(Debug)]
struct BiosParameterBlock {
    /// 每扇区字节数
    /// 512/1024/2048/4096
    bytes_per_sector: u16,
    /// 每簇扇区数
    /// 1/2/4/8/16/32/64/128
    sectors_per_cluster: u8,
    /// 保留区中保留扇区的数目，保留扇区从 FAT 卷的第一个扇区开始
    /// 对于 FAT12 和 FAT16 必须为 1，FAT32 的 典 型 取 值 为 32,
    reserved_sectors: u16,
    /// 此卷中 FAT 表的份数。
    /// recommended: 2
    number_of_fats: u8,
    /// 对于 FAT12 和 FAT16 此域包含根目录中的目录项数（每个项长度为 32 bytes），
    /// 对于 FAT32,此项必须为 0
    root_dir_entries: u16,
    ///  16-bit 的总扇区数，这里的总扇区数包括 FAT卷上四个基本区的全部扇区
    /// fat32: 0
    total_sectors: u16,
    /// media descriptor
    /// 0xf0/0xf8/0xf9/0xfa/0xfb/0xfc/0xfd/0xfe/0xff
    media_descriptor: u8,
    /// FAT12/FAT16 一个 FAT 表所占的扇区数，对于 FAT32 此域必须为零
    /// fat32: 0
    sectors_per_fat_16: u16,
    /// 每磁道扇区数
    /// 0
    sectors_per_track: u16,
    /// 磁 头 数
    /// 0
    number_of_heads: u16,
    ///在此 FAT 分区之前所隐藏的扇区数
    /// 0
    hidden_sectors: u32,
    /// 该卷总扇区数（32-bit），这里的总扇区数包括 FAT 卷上四个基本区的全部扇区
    /// fat32: not 0
    total_sectors_32: u32,
    /// 一个 FAT 表所占的扇区数，此域为 FAT32 特有
    sectors_per_fat_32: u32,
    /// for fat32
    /// bits0-3: active FAT
    /// bits4-6: 0
    /// bit7: 1
    ext_flags: u16,
    file_system_version: u16,
    /// 根目录所在第一个簇的簇号，通常该数值为 2，但不是必须为 2
    root_dir_cluster: u32,
    /// file system information sector
    /// 1
    file_system_info_sector: u16,
    /// 此域 FAT32 特有。如果不为 0，表示在保留区中引导记录的
    /// 备份数据所在的扇区，通常为 6
    backup_boot_sector: u16,
    reserved: [u8; 12],
    /// 0x00/0x80
    driver_number: u8,
    reserved1: u8,
    boot_signature: u8,
    /// volume serial number
    /// time+day
    volume_serial_number: u32,
    /// 磁盘卷标，此域必须与根目录中 11 字节长的卷标一致。
    /// NOTE： FAT 文件系统必须保证在根目录的卷标文件更改或
    /// 是创建的同时，此域的内容能得到及时的更新，当 FAT 卷没
    /// 有卷标时，此域的内容为“NO NANM ”
    volume_label: [u8; 11],
    /// file system type
    /// 请勿使用此字段进行文件系统类型判断
    file_system_type: [u8; 8],
}

struct Dbr {
    /// jump code
    jump: [u8; 3],
    /// name
    oem: [u8; 8],
    /// BIOS parameter block
    bpb: BiosParameterBlock,
}
