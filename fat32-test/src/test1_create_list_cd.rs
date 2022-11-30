use fat32_trait::DirectoryLike;
use std::error::Error;
use std::sync::Arc;

pub fn test1_create_list_cd(root: Arc<dyn DirectoryLike<Error: Error  + 'static>>) {
    test_create_file_and_dir(root.clone());
    test_create_fail(root.clone());
    test_cd_dir(root.clone());
    test_multi_cd_dir(root.clone());
}

fn test_create_file_and_dir(root: Arc<dyn DirectoryLike<Error: Error  + 'static>>) {
    let ans = root.create_file("test_create_file_and_dir");
    assert!(ans.is_ok());
    let ans = root.create_dir("test_create_file_and_dir");
    assert!(ans.is_ok());
    let names = root.list().unwrap();
    assert!(names.contains(&"test_create_file_and_dir".to_string()));
    println!("test_create_file_and_dir passed");
}

fn test_create_fail(root: Arc<dyn DirectoryLike<Error: Error  + 'static>>) {
    let ans = root.create_file("test_create_fail");
    assert!(ans.is_ok());
    let ans = root.create_file("test_create_fail");
    assert!(ans.is_err());
    let ans = root.create_dir("test_create_fail");
    assert!(ans.is_ok());
    let ans = root.create_dir("test_create_fail");
    assert!(ans.is_err());
    println!("test_create_fail passed");
}

fn test_cd_dir(root: Arc<dyn DirectoryLike<Error: Error  + 'static>>) {
    let ans = root.create_dir("test_cd_dir");
    assert!(ans.is_ok());
    let ans = root.cd("test_cd_dir");
    assert!(ans.is_ok());
    let test_cd_dir = ans.unwrap();
    let ans = test_cd_dir.cd("temp");
    assert!(ans.is_err());
    println!("test_cd_dir passed");
}

fn test_multi_cd_dir(root: Arc<dyn DirectoryLike<Error: Error  + 'static>>) {
    let ans = root.create_dir("test_multi_cd_dir");
    assert!(ans.is_ok());
    let ans = root.cd("test_multi_cd_dir");
    assert!(ans.is_ok());
    let test_multi_cd_dir = ans.unwrap();
    let ans = test_multi_cd_dir.create_dir("temp");
    assert!(ans.is_ok());
    let ans = test_multi_cd_dir.cd("temp");
    assert!(ans.is_ok());
    let temp = ans.unwrap();
    let ans = temp.cd("temp");
    assert!(ans.is_err());
    println!("test_multi_cd_dir passed");
}

fn test_multi_thread_create(root: Arc<dyn DirectoryLike<Error: Error  + 'static>>) {
    let ans = root.create_dir("test_multi_thread_create");
    assert!(ans.is_ok());
    let root_thread = root.clone();
    let thread = std::thread::spawn(move || {
        let ans = root_thread.clone().create_file("test_multi_thread_create");
        assert!(ans.is_err());
    });
    let mut threads = Vec::new();
    for i in 0..4 {
        let root_thread = root.clone();
        let thread = std::thread::spawn(move || {
            let ans = root_thread.create_file(&format!("test_multi_thread_create{}", i));
            assert!(ans.is_ok());
        });
        threads.push(thread);
    }
    thread.join().unwrap();
    for thread in threads {
        thread.join().unwrap();
    }
    let files = [
        "test_multi_thread_create",
        "test_multi_thread_create0",
        "test_multi_thread_create1",
        "test_multi_thread_create2",
        "test_multi_thread_create3",
    ];
    let list = root.list().unwrap();
    for file in files.iter() {
        assert!(list.contains(&file.to_string()));
    }
    println!("test_multi_thread_create passed");
}
