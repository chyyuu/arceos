#![no_std]
#![no_main]

use alloc::{vec::Vec, string::String};
use libax::info;

extern crate alloc;

fn test_directory() {
    // create a test directory
    libax::fs::create_dir("/totop".into()).expect("can't create directory");

    // whether the test directory exists
    let finded: Vec<String> = libax::fs::read_dir("/".into()).expect("can't read directory")
        .into_iter().filter(|x| x == "totop").collect();
    assert_eq!(finded.len(), 1);

    // remove the directory
    libax::fs::remove_dir("/totop".into()).expect("can't remove directory");
}

fn test_list_files() {
    // list files in the root directory
    libax::println!("{:=^30}", " file list ");
    libax::fs::read_dir("/".into()).map(|x| {
        for file_name in x {
            libax::println!("{}", file_name);
        }
    }).expect("can't read root directory");
    libax::println!("{:=^30}", " file list end ");
}

fn test_file() {
    // write a test file, if the file not exists, then create it
    libax::fs::write("/test.txt".into(), b" Hello fs\n")
        .expect("can't write to test file");

    // read the file from the file
    let file_content = libax::fs::read("/test.txt".into()).expect("can't read the test file");
    assert_eq!(file_content, b" Hello fs\n");

    // whether the file exists
    let finded: Vec<String> = libax::fs::read_dir("/".into()).expect("can't read directory")
        .into_iter().filter(|x| x == "test.txt").collect();
    assert_eq!(finded.len(), 1);

    // remove the file
    libax::fs::remove_file("/test.txt".into()).expect("can't remove test file");
}

#[no_mangle]
fn main() {
    libax::println!("Hello, world!");

    test_list_files();
    test_directory();
    test_file();

    // if let Some(file) = fs.root().open("hellofs.txt") {
    //     let mut buf = vec![0u8; 300];
    //     let file_size = file.read(&mut buf);
    //     println!("file content: {}", String::from_utf8_lossy(&buf[..file_size]));
    // }

    // if let Some(fs) = filesystems().first() {
    //     if let Some(file) = fs.root().create("hellofs.txt") {
    //         file.seek(vfscore::SeekFrom::End(0));
    //         file.write(b" Hello fs\n");
    //     }

    //     println!("{:=^30}", " file list ");
    //     for f_name in fs.root().read_dir() {
    //         println!("{}", f_name);
    //     }
    //     println!("{:=^30}", " file list end ");

    //     if let Some(file) = fs.root().open("hellofs.txt") {
    //         let mut buf = vec![0u8; 300];
    //         let file_size = file.read(&mut buf);
    //         println!("file content: {}", String::from_utf8_lossy(&buf[..file_size]));
    //     }
    // }
}
