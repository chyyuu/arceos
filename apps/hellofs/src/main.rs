#![no_std]
#![no_main]

use axfs::filesystems;

#[macro_use]
extern crate axruntime;

#[no_mangle]
fn main() {
    println!("Hello, world!");
    if let Some(fs) = filesystems().first() {
        println!("file list");
        for f_name in fs.root().read_dir() {
            println!("{}", f_name);
        }
    }
}
