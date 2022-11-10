use crate::device::DEVICE;
use crate::utils::BLOCK_SIZE;
use alloc::collections::VecDeque;
use alloc::sync::Arc;
use log::error;
use spin::{Once, RwLock};

/// 需要使用读写锁保护数据，防止多个线程同时访问
pub struct BlockCache {
    id: usize,
    inner: RwLock<BlockCacheInner>,
}

struct BlockCacheInner {
    dirty: bool,
    data: [u8; BLOCK_SIZE],
}

impl BlockCache {
    pub fn new(block_id: usize, data: [u8; BLOCK_SIZE]) -> Self {
        Self {
            id: block_id,
            inner: RwLock::new(BlockCacheInner { dirty: false, data }),
        }
    }

    fn addr_of_offset(&self, offset: usize) -> usize {
        let inner = self.inner.read();
        let s = &(*inner).data[offset..offset + 4];
        let x = &(*inner).data[offset] as *const _ as usize;
        x
    }

    pub fn get_ref<T>(&self, offset: usize) -> &T
    where
        T: Sized,
    {
        let type_size = core::mem::size_of::<T>();
        assert!(offset + type_size <= BLOCK_SIZE);
        let addr = self.addr_of_offset(offset);
        unsafe { &*(addr as *const T) }
    }
    pub fn get_mut<T>(&self, offset: usize) -> &mut T
    where
        T: Sized,
    {
        let type_size = core::mem::size_of::<T>();
        assert!(offset + type_size <= BLOCK_SIZE);
        let addr = self.addr_of_offset(offset);
        let mut inner = self.inner.write();
        (*inner).dirty = true;
        unsafe { &mut *(addr as *mut T) }
    }

    pub fn read<T, V>(&self, offset: usize, f: impl FnOnce(&T) -> V) -> V {
        f(self.get_ref(offset))
    }

    pub fn write<T, V>(&self, offset: usize, f: impl FnOnce(&mut T) -> V) -> V {
        f(self.get_mut(offset))
    }

    pub fn sync(&self) {
        let inner = self.inner.read();
        let data = ((*inner).data).as_ref();
        error!("sync block {}", self.id);
        if inner.dirty {
            DEVICE.get().unwrap().lock().write(self.id, data).unwrap();
        }
    }
}

impl Drop for BlockCache {
    fn drop(&mut self) {
        self.sync()
    }
}

pub static mut CACHE_MANAGER: Once<Box<dyn Cache>> = Once::new();

pub struct CacheManager {
    cache: VecDeque<Arc<BlockCache>>,
    size: usize,
}

pub trait Cache: Send + Sync {
    fn get_cache_by_id(&mut self, id: usize) -> Arc<BlockCache>;
    fn sync(&self);
}

impl CacheManager {
    pub fn new(size: usize) -> Self {
        CacheManager {
            cache: VecDeque::new(),
            size,
        }
    }
}

impl Cache for CacheManager {
    fn get_cache_by_id(&mut self, id: usize) -> Arc<BlockCache> {
        let ans = self.cache.iter().find(|&cache| cache.id == id);
        match ans {
            Some(cache) => cache.clone(),
            None => {
                if self.cache.len() == self.size {
                    self.cache.pop_front();
                }
                let mut buffer = [0u8; BLOCK_SIZE];
                DEVICE.get().unwrap().lock().read(id, &mut buffer).unwrap();
                let cache = Arc::new(BlockCache::new(id, buffer));
                self.cache.push_back(cache.clone());
                cache
            }
        }
    }
    fn sync(&self) {
        for cache in self.cache.iter() {
            cache.sync();
        }
    }
}

pub fn get_block_cache_by_id(block_id: usize) -> Arc<BlockCache> {
    unsafe { CACHE_MANAGER.get_mut().unwrap().get_cache_by_id(block_id) }
}

pub fn sync(){
    unsafe { CACHE_MANAGER.get_mut().unwrap().sync() }
}
