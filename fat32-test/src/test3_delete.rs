use fat32_trait::DirectoryLike;

use std::error::Error;

use std::sync::Arc;

pub fn test_delete_file_and_dir(root: Arc<dyn DirectoryLike<Error: Error  + 'static>>) {
    test_delete_file(root.clone());
    test_delete_dir(root.clone());
}

fn test_delete_file(root: Arc<dyn DirectoryLike<Error: Error  + 'static>>) {
    let ans = root.create_file("test_delete_file");
    assert!(ans.is_ok());
    let ans = root.delete_file("test_delete_file");
    assert!(ans.is_ok());
    let ans = root.delete_file("test_delete_file");
    assert!(ans.is_err());
    let ans = root.delete_dir("test_delete_file");
    assert!(ans.is_err());
    let ans = root.open("test_delete_file");
    assert!(ans.is_err());
    for i in 0..100 {
        let ans = root.create_file(&format!("test_delete_file{}", i));
        assert!(ans.is_ok());
    }
    for i in 0..100 {
        let ans = root.delete_file(&format!("test_delete_file{}", i));
        assert!(ans.is_ok());
    }
    let ans = root.create_file("test_delete_file");
    assert!(ans.is_ok());
    let file = root.open("test_delete_file").unwrap();
    file.write(0, &[0; 512]).unwrap();
    root.delete_file("test_delete_file").unwrap();
    let ans = root.open("test_delete_file");
    assert!(ans.is_err());
    println!("test_delete_file passed");
}

fn test_delete_dir(root: Arc<dyn DirectoryLike<Error: Error  + 'static>>) {
    root.create_dir("test_delete_dir").unwrap();
    let dir = root.cd("test_delete_dir").unwrap();
    dir.create_dir("sub_test_delete_dir").unwrap();
    let sub_dir = dir.cd("sub_test_delete_dir").unwrap();
    sub_dir.create_file("test_delete_dir").unwrap();
    let file = sub_dir.open("test_delete_dir").unwrap();
    file.write(0, &[0; 512]).unwrap();
    let content = file.read(0, 512).unwrap();
    assert_eq!(content, [0; 512]);
    dir.delete_dir("sub_test_delete_dir").unwrap();
    let file_and_dir = dir.list().unwrap();
    assert_eq!(file_and_dir.len(), 2);
    assert!(file_and_dir.contains(&".".to_string()));
    assert!(file_and_dir.contains(&"..".to_string()));
    println!("test_delete_dir passed");
}
