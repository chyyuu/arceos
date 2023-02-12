#![no_std]
#![no_main]

use alloc::string::String;
use axfs::filesystems;

#[macro_use]
extern crate alloc;
#[macro_use]
extern crate axruntime;

#[no_mangle]
fn main() {
    println!("Hello, world!");
    if let Some(fs) = filesystems().first() {
        if let Some(file) = fs.root().create("hellofs.txt") {
            file.write(b"Hello fs");
        }

        println!("{:=^30}", " file list ");
        for f_name in fs.root().read_dir() {
            println!("{}", f_name);
        }
        println!("{:=^30}", " file list end ");

        if let Some(file) = fs.root().open("hellofs.txt") {
            let mut buf = vec![0u8; 300];
            let file_size = file.read(&mut buf);
            println!("file content: {}", String::from_utf8_lossy(&buf[..file_size]));
        }
    }
}
