use fscommon::BufStream;
use std::fs::OpenOptions;

use std::io::{self, prelude::*};

use fatfs::{FileSystem, FsOptions};

pub fn test_second_fat32() -> io::Result<()> {
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .open("fat32-test/test.img")
        .unwrap();
    let buf_rdr = BufStream::new(file);
    let fs = FileSystem::new(buf_rdr, FsOptions::new())?;
    let root_dir = fs.root_dir();
    root_dir.create_dir("test_test_test")?;
    root_dir.create_file("test.txt")?;
    root_dir.iter().for_each(|name| {
        if name.is_ok() {
            let t = name.unwrap();
            let x = t.file_name();
            println!("{x}");
        }
    });
    let mut file = root_dir.open_file("test.txt")?;
    let mut buf = vec![];
    file.read_to_end(&mut buf)?;
    print!("{}", String::from_utf8_lossy(&buf));
    Ok(())
}

fn main(){

}