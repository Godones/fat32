use crate::cache::{sync, CacheManager, CACHE_MANAGER};
use crate::device::{BlockDevice, DEVICE};
use crate::dir::Dir;
use crate::utils::{u16_from_le_bytes, u32_from_le_bytes, BLOCK_SIZE};
use crate::{block_buffer, Fat, FsInfo, MetaData};
use alloc::sync::Arc;
use core::fmt::Debug;
use log::error;
use spin::{Mutex, RwLock};

#[derive(Debug)]
pub struct Fat32 {
    root_dir: Arc<Dir>,
}

impl Fat32 {
    pub fn new<T: BlockDevice>(device: T) -> Result<Fat32, ()> {
        // 需要读取第一扇区构建原始信息
        let mut buffer = block_buffer!();
        let _dbr = device.read(0, &mut buffer).unwrap();
        // todo!忽略了正确性检查
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
            (0, 0),
            meta_data,
            Arc::new(RwLock::new(fat)),
        );
        Ok(Fat32 {
            root_dir: Arc::new(root_dir),
        })
    }
    pub fn root_dir(&self) -> Arc<Dir> {
        self.root_dir.clone()
    }
    pub fn sync(&self) {
        sync();
    }
}
