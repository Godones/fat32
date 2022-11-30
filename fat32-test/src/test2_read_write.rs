use fat32_trait::DirectoryLike;
use mfat32::Dir;
use std::error::Error;
use std::fmt::Debug;
use std::sync::Arc;

pub fn test2_read_write(root: Arc<dyn DirectoryLike<Error: Error + Debug + 'static>>) {
    test_read_empty_file(root.clone());
    test_write_small_file(root.clone());
    test_write_large_file(root.clone());
    test_read_multi_thread(root.clone());
    test_write_multi_thread(root.clone());
    test_clear_file(root.clone());
}

fn test_read_empty_file(root: Arc<dyn DirectoryLike<Error: Error + Debug + 'static>>) {
    let ans = root.create_file("test_read_file");
    assert!(ans.is_ok());
    let test_read_file = root.open("test_read_file");
    assert!(test_read_file.is_ok());
    let test_read_file = test_read_file.unwrap();
    let ans = test_read_file.read(0, 10);
    assert!(ans.is_ok());
    let ans = ans.unwrap();
    assert_eq!(ans.len(), 0);
    let ans = root.open("no_file");
    assert!(ans.is_err());
    println!("test_read_empty_file passed");
}

fn test_write_small_file(root: Arc<dyn DirectoryLike<Error: Error + Debug + 'static>>) {
    let ans = root.create_file("test_write_file");
    assert!(ans.is_ok());
    let test_write_file = root.open("test_write_file");
    assert!(test_write_file.is_ok());
    let mut test_write_file = test_write_file.unwrap();
    let ans = test_write_file.write(0, &[1, 2, 3, 4, 5]);
    assert!(ans.is_ok());
    let content = test_write_file.read(0, 10);
    assert!(content.is_ok());
    assert_eq!(content.unwrap(), [1, 2, 3, 4, 5]);
    let w_content = [0x12; 512];
    let ans = test_write_file.write(0, &w_content);
    assert!(ans.is_ok());
    let content = test_write_file.read(0, 10);
    assert!(content.is_ok());
    let content = content.unwrap();
    assert_eq!(content.len(), 10);
    assert_eq!(content, w_content[0..10]);
    println!("test_write_small_file passed");
}

fn test_write_large_file(root: Arc<dyn DirectoryLike<Error: Error + Debug + 'static>>) {
    let ans = root.create_file("test_write_large_file");
    assert!(ans.is_ok());
    let test_write_large_file = root.open("test_write_large_file");
    let data = [0x12; 512 * 10];
    let mut test_write_large_file = test_write_large_file.unwrap();
    test_write_large_file.write(0, &data);
    let content = test_write_large_file.read(512, 10);
    assert!(content.is_ok());
    let content = content.unwrap();
    assert_eq!(content.len(), 10);
    assert_eq!(content, data[512..512 + 10]);
    println!("test_write_large_file passed");
    let content = test_write_large_file.read(512 * 10 - 10, 10);
    assert!(content.is_ok());
    let content = content.unwrap();
    assert_eq!(content.len(), 10);
    assert_eq!(content, data[512..512 + 10]);
}

fn test_read_multi_thread(root: Arc<dyn DirectoryLike<Error: Error + Debug + 'static>>) {
    let ans = root.create_file("test_read_multi_thread");
    assert!(ans.is_ok());
    let test_read_multi_thread = root.open("test_read_multi_thread");
    assert!(test_read_multi_thread.is_ok());
    let test_read_multi_thread = test_read_multi_thread.unwrap();
    test_read_multi_thread.write(0, &[0x22; 512]);
    let threads = (0..10)
        .map(|_| {
            let test_read_multi_thread = test_read_multi_thread.clone();
            std::thread::spawn(move || {
                let content = test_read_multi_thread.read(0, 10);
                assert!(content.is_ok());
                let content = content.unwrap();
                assert_eq!(content.len(), 10);
                assert_eq!(content, [0x22; 10]);
            })
        })
        .collect::<Vec<_>>();
    println!("test_read_multi_thread passed");
}

fn test_write_multi_thread(root: Arc<dyn DirectoryLike<Error: Error + Debug + 'static>>) {
    let ans = root.create_file("test_write_multi_thread");
    assert!(ans.is_ok());
    let test_write_multi_thread = root.open("test_write_multi_thread");
    assert!(test_write_multi_thread.is_ok());
    let test_write_multi_thread = test_write_multi_thread.unwrap();
    let threads = (0..10)
        .map(|i| {
            let test_write_multi_thread = test_write_multi_thread.clone();
            std::thread::spawn(move || {
                let ans = test_write_multi_thread.write(0, &[i; 512]);
                assert!(ans.is_ok());
            })
        })
        .collect::<Vec<_>>();
    for thread in threads {
        thread.join().unwrap();
    }
    let content = test_write_multi_thread.read(0, 512).unwrap();
    let may_be_content = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9];
    let find = may_be_content.iter().find(|x| content.contains(x));
    assert!(find.is_some());
    let val = find.unwrap();
    assert_eq!(content, [*val; 512]);
    println!("test_write_multi_thread passed");
}

fn test_clear_file(root: Arc<dyn DirectoryLike<Error: Error + Debug + 'static>>) {
    let ans = root.create_file("test_clear_file").unwrap();
    let test_clear_file = root.open("test_clear_file");
    assert!(test_clear_file.is_ok());
    let mut test_clear_file = test_clear_file.unwrap();
    test_clear_file.write(0, &[0x12; 512]);
    test_clear_file.clear();
    let content = test_clear_file.read(0, 512).unwrap();
    assert_eq!(content.len(), 0);
    println!("test_clear_file passed");
}
