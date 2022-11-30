use fat32_trait::DirectoryLike;
use mfat32::Dir;
use std::error::Error;
use std::fmt::Debug;
use std::sync::Arc;

pub fn test_rename(root: Arc<dyn DirectoryLike<Error: Error  + 'static>>) {
    let new_dir = root.create_dir("test_rename");
    assert!(new_dir.is_ok());
    let root = root.cd("test_rename").unwrap();
    root.create_file("test.txt").unwrap();
    let a = root.rename_file("test.txt", "newtest.txt");
    assert!(a.is_ok());
    let a = root.create_dir("test_dir").unwrap();
    let a = root.rename_dir("test_dir", "new_test_dir");
    assert!(a.is_ok());
    let names = root.list().unwrap();
    assert!(names.contains(&"newtest.txt".to_string()));
    assert!(names.contains(&"new_test_dir".to_string()));
    assert_eq!(names.len(), 4);
    root.create_file("test.txt").unwrap();
    root.create_dir("test_dir").unwrap();
    println!("test_rename passed");
}
