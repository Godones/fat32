use std::time;
use bit_field::BitField;
use crate::dir::Dir;
use bitflags::bitflags;
bitflags! {
    pub struct EntryFlags:u8{
        const READ_ONLY = 0b0000_0001;
        const HIDDEN = 0b0000_0010;
        const SYSTEM = 0b0000_0100;
        const VOLUME_ID = 0b0000_1000;
        const DIRECTORY = 0b0001_0000;
        const ARCHIVE = 0b0010_0000;
        const LONG_NAME = Self::READ_ONLY.bits | Self::HIDDEN.bits | Self::SYSTEM.bits | Self::VOLUME_ID.bits;
    }
}

/// 短目录项
#[derive(Debug)]
pub struct ShortEntry {
    name: [u8; 8],
    ext: [u8; 3],
    attr: EntryFlags,
    reserved: u8,
    create_time: u16,
    create_date: u16,
    last_access_date: u16,
    cluster_high: u16,
    modify_time: u16,
    modify_date: u16,
    cluster_low: u16,
    file_size: u32,
}

#[derive(Debug)]
pub struct FullLoongEntry {
    entries: Vec<LongEntry>,
}

impl FullLoongEntry {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }
    pub fn push(&mut self, entry: LongEntry) {
        self.entries.push(entry);
    }
    pub fn filename(&self) -> String {
        let mut filename = String::new();
        for entry in self.entries.iter() {
            filename.push_str(&entry.filename());
        }
        filename
    }
    pub fn clear(&mut self) {
        self.entries.clear();
    }
    pub fn from_file_name(filename: &str, check_sum: u8) -> Self {
        let mut entries = Vec::new();
        let mut filename = filename.to_string();
        let mut index = 1;
        while filename.len() > 13 {
            let name = filename.split_off(13);
            let mut entry = LongEntry::new(&filename, index, check_sum);
            entries.push(entry);
            filename = name;
        }
        // 不满足13个字符的最后一个entry
        // 其order为0x40|index
        let mut entry = LongEntry::new(&filename, 0x40 | index, check_sum);
        entries.push(entry);
        Self { entries }
    }
    pub fn len(&self) -> usize {
        self.entries.len()
    }
    pub fn iter(&self) -> std::slice::Iter<LongEntry> {
        self.entries.iter()
    }
}

/// 长目录项
#[derive(Debug)]
pub struct LongEntry {
    order: u8,
    name1: [u16; 5],
    attr: EntryFlags,
    reserved: u8,
    checksum: u8,
    name2: [u16; 6],
    cluster: u16,
    name3: [u16; 2],
}

impl ShortEntry {
    pub fn new(name: &str, attr: EntryFlags, cluster: u32) -> Self {
        /// 长文件名需要截断并全部转换为大写
        let (name,ext) = if name == "." || name==".."{
            (name.to_string(),String::new())
        }else {
            let dot_index = name.rfind(".").unwrap_or(name.len());
            let name = name[..dot_index].to_uppercase();
            let ext = name[dot_index..].to_uppercase();
            (name,ext)
        };
        let mut buffer = [0u8; 32];
        // 用空格填充
        buffer[0..8].fill(0x20);
        buffer[8..11].fill(0x20);
        // 写入文件名
        buffer[0..name.len()].copy_from_slice(name.as_bytes());
        // 写入扩展名
        buffer[8..8 + ext.len()].copy_from_slice(ext.as_bytes());
        buffer[11] = attr.bits();
        // 写入起始簇号
        buffer[26..28].copy_from_slice(&cluster.to_le_bytes()[0..2]);
        buffer[20..22].copy_from_slice(&cluster.to_le_bytes()[2..4]);

        //写入创建的时间
        buffer[13] = 124;
        buffer[14] = 188;
        buffer[15]=  108;
        buffer[16] = 106;
        buffer[17] =  85;
        buffer[18] =  106;
        buffer[19] =  85;
        buffer[22] =  188;
        buffer[23] =  108;
        buffer[24] =  106;
        buffer[25] =  85;
        
        let short_entry = ShortEntry::from_buffer(&buffer);
        short_entry
    }
    /// 从字节数组中解析出短目录项
    pub fn from_buffer(buffer: &[u8]) -> Self {
        let mut name = [0u8; 8];
        let mut ext = [0u8; 3];
        name.copy_from_slice(&buffer[0..8]);
        ext.copy_from_slice(&buffer[8..11]);
        let attr = EntryFlags::from_bits_truncate(buffer[11]);
        let reserved = buffer[12];
        let create_time = u16::from_le_bytes([buffer[13], buffer[14]]);
        let create_date = u16::from_le_bytes([buffer[15], buffer[16]]);
        let last_access_date = u16::from_le_bytes([buffer[17], buffer[18]]);
        let cluster_high = u16::from_le_bytes([buffer[20], buffer[21]]);
        let modify_time = u16::from_le_bytes([buffer[22], buffer[23]]);
        let modify_date = u16::from_le_bytes([buffer[24], buffer[25]]);
        let cluster_low = u16::from_le_bytes([buffer[26], buffer[27]]);
        let file_size = u32::from_le_bytes([buffer[28], buffer[29], buffer[30], buffer[31]]);
        Self {
            name,
            ext,
            attr,
            reserved,
            create_time,
            create_date,
            last_access_date,
            cluster_high,
            modify_time,
            modify_date,
            cluster_low,
            file_size,
        }
    }
    pub fn to_buffer(&self) -> [u8; 32] {
        let mut buffer = [0u8; 32];
        buffer[0..8].copy_from_slice(&self.name);
        buffer[8..11].copy_from_slice(&self.ext);
        buffer[11] = self.attr.bits();
        buffer[12] = self.reserved;
        buffer[13..15].copy_from_slice(&self.create_time.to_le_bytes());
        buffer[15..17].copy_from_slice(&self.create_date.to_le_bytes());
        buffer[17..19].copy_from_slice(&self.last_access_date.to_le_bytes());
        buffer[20..22].copy_from_slice(&self.cluster_high.to_le_bytes());
        buffer[22..24].copy_from_slice(&self.modify_time.to_le_bytes());
        buffer[24..26].copy_from_slice(&self.modify_date.to_le_bytes());
        buffer[26..28].copy_from_slice(&self.cluster_low.to_le_bytes());
        buffer[28..32].copy_from_slice(&self.file_size.to_le_bytes());
        buffer
    }
    /// 去掉0x20所占的字符
    pub fn filename(&self) -> String {
        let mut name = String::new();
        for &byte in self.name.iter() {
            if byte == 0x20 || byte == 0x00 {
                break;
            }
            name.push(byte as char);
        }
        if self.ext[0] != 0x20 {
            name.push('.');
            for &byte in self.ext.iter() {
                if byte == 0x20 || byte == 0x00 {
                    break;
                }
                name.push(byte as char);
            }
        }
        name
    }

    /// 计算短文件名的校验和
    pub fn checksum(&self) -> u8 {
        let mut sum = 0u8;
        for &byte in self.name.iter() {
            sum = ((sum & 1) << 7) + (sum >> 1) + byte;
        }
        for &byte in self.ext.iter() {
            sum = ((sum & 1) << 7) + (sum >> 1) + byte;
        }
        sum
    }

    pub fn attr(&self) -> &EntryFlags {
        &self.attr
    }
    pub fn start_cluster(&self) -> u32 {
        u32::from(self.cluster_high) << 16 | u32::from(self.cluster_low)
    }
    pub fn file_size(&self) -> u32 {
        self.file_size
    }
}

impl LongEntry {
    pub fn new(name: &str, order: u8, checksum: u8) -> Self {
        let mut name1 = [0u16; 5];
        let mut name2 = [0u16; 6];
        let mut name3 = [0u16; 2];
        let mut utf16 = name.encode_utf16().collect::<Vec<u16>>();
        if utf16.len() < 13 {
            let len = utf16.len();
            for _ in 0..13 - len {
                utf16.push(0xFFFF);
            }
        }
        name1.copy_from_slice(&utf16[0..5]);
        name2.copy_from_slice(&utf16[5..11]);
        name3.copy_from_slice(&utf16[11..13]);
        Self {
            order,
            name1,
            attr: EntryFlags::LONG_NAME,
            reserved: 0,
            checksum,
            name2,
            cluster: 0,
            name3,
        }
    }
    pub fn from_buffer(buffer: &[u8]) -> Self {
        let order = buffer[0];
        let mut name1 = [0u16; 5];
        let mut name2 = [0u16; 6];
        let mut name3 = [0u16; 2];
        name1.copy_from_slice(
            &buffer[1..11]
                .chunks(2)
                .map(|c| u16::from_le_bytes([c[0], c[1]]))
                .collect::<Vec<u16>>()[..],
        );
        let attr = EntryFlags::from_bits_truncate(buffer[11]);
        let reserved = buffer[12];
        let checksum = buffer[13];
        name2.copy_from_slice(
            &buffer[14..26]
                .chunks(2)
                .map(|c| u16::from_le_bytes([c[0], c[1]]))
                .collect::<Vec<u16>>()[..],
        );
        let cluster = u16::from_le_bytes([buffer[26], buffer[27]]);
        name3.copy_from_slice(
            &buffer[28..32]
                .chunks(2)
                .map(|c| u16::from_le_bytes([c[0], c[1]]))
                .collect::<Vec<u16>>()[..],
        );
        Self {
            order,
            name1,
            attr,
            reserved,
            checksum,
            name2,
            cluster,
            name3,
        }
    }
    pub fn filename(&self) -> String {
        let mut name = Vec::new();
        self.name1.iter().for_each(|c| {
            if *c != 0xFFFF as u16 && *c != 0x0000 as u16 {
                name.push(*c)
            } else {
                return;
            }
        });
        self.name2.iter().for_each(|c| {
            if *c != 0xFFFF as u16 && *c != 0x0000 as u16 {
                name.push(*c)
            } else {
                return;
            }
        });
        self.name3.iter().for_each(|c| {
            if *c != 0xFFFF as u16 && *c != 0x0000 as u16 {
                name.push(*c)
            } else {
                return;
            }
        });

        String::from_utf16(&name).unwrap()
    }

    pub fn to_buffer(&self) -> [u8; 32] {
        let mut buffer = [0u8; 32];
        buffer[0] = self.order;
        buffer[1..11].copy_from_slice(
            &self
                .name1
                .iter()
                .flat_map(|c| c.to_le_bytes().to_vec())
                .collect::<Vec<u8>>()[..],
        );
        buffer[11] = self.attr.bits();
        buffer[12] = self.reserved;
        buffer[13] = self.checksum;
        buffer[14..26].copy_from_slice(
            &self
                .name2
                .iter()
                .flat_map(|c| c.to_le_bytes().to_vec())
                .collect::<Vec<u8>>()[..],
        );
        buffer[26..28].copy_from_slice(&self.cluster.to_le_bytes());
        buffer[28..32].copy_from_slice(
            &self
                .name3
                .iter()
                .flat_map(|c| c.to_le_bytes().to_vec())
                .collect::<Vec<u8>>()[..],
        );
        buffer
    }
}
