#![cfg_attr(not(test), no_std)]

extern crate alloc;

use alloc::boxed::Box;
use alloc::vec::Vec;
use alloc::string::String;

pub trait VfsFile {
    fn read_dir(&self) -> Vec<String>;
    fn read(&self, buf: &mut [u8]) -> usize;
    fn write(&self, data: &[u8]) -> usize;
    fn close(&self);
}

pub trait VfsFileSystem: Send + Sync {
    fn name(&'static self) -> &'static str;
    fn root(&'static self) -> Box<dyn VfsFile>;
    fn open(&'static self, path: &str) -> Option<Box<dyn VfsFile>>;
}

pub trait DiskOperation {
    fn read_block(index: usize, buf: &mut [u8]);
    fn write_block(index: usize, data: &[u8]);
}