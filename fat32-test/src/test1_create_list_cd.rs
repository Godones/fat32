use std::sync::Arc;
use mfat32::{Dir, DirectoryLike};


fn test1_create_list_cd(root:Arc<Dir>){
    test_create_file_and_dir(root.clone());
    test_create_fail(root.clone());
    test_cd_dir(root.clone());
}

fn test_create_file_and_dir(root:Arc<Dir>){
    let ans = root.create_file("test_create_file_and_dir");
    assert!(ans.is_ok());
    let ans = root.create_file("test_create_file_and_dir");
    assert!(ans.is_ok());
    let names = root.list().unwrap();
    assert!(names.contains(&"test_create_file_and_dir".to_string()));
}

fn test_create_fail(root:Arc<Dir>){
    let ans = root.create_file("test_create_fail");
    assert!(ans.is_ok());
    let ans = root.create_file("test_create_fail");
    assert!(ans.is_err());
    let ans = root.create_dir("test_create_fail");
    assert!(ans.is_ok());
    let ans = root.create_dir("test_create_fail");
    assert!(ans.is_err());
}

fn test_cd_dir(root:Arc<Dir>){
    let ans = root.create_dir("test_cd_dir");
    assert!(ans.is_ok());
    let ans = root.cd("test_cd_dir");
    assert!(ans.is_ok());
    let test_cd_dir = ans.unwrap();
    let ans = test_cd_dir.cd("temp");
    assert!(ans.is_err());
}

