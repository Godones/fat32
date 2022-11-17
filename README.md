# Fat32文件系统

作为个人项目，根据fat32手册以及网上相关说明实现fat32文件系统，与`linux`系统创建的fat32兼容。

## 特征

- [x] 短文件名
- [x] 长文件名
- [x] 创建文件/创建文件夹
- [x] 删除文件/删除文件夹
- [x] 读取文件内容/清空文件内容
- [x] 写入文件内容
- [x] ls/cd
- [x] 重命名
- [x] tests

## 接口规范

### For Dir

```rust
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
```

### For File

```rust
pub trait FileLike {
    type Error;
    fn read(&self, offset: u32, size: u32) -> Result<Vec<u8>, Self::Error>;
    fn write(&self, offset: u32, data: &[u8]) -> Result<u32, Self::Error>;
    fn clear(&self);
}
```



## 使用

```rust
use fat32::{BlockDevice, DirectoryLike, Fat32};
let device = FakeDevice::new("fat32-test/test.img");
let fat32 = Fat32::new(device).unwrap();
let root = fat32.root_dir();
let _ans = root.create_file("test.txt");
let ans = root.create_dir("test");
root.list().unwrap().iter().for_each(|name| {
    println!("{}", name);
});
fat32.sync();
```



`examples`目录下有简单的使用案例，更多的使用方式可以查看`fat32-test`目录下的测试。`fat32-test`目录下有创建`fat32`文件的`Makefile`脚本。使用前请运行脚本。如果想查看示例的效果，需要重新挂载文件系统，使用命令`make umount && make mount`后进入`/fat`查看是否正确创建文件。

