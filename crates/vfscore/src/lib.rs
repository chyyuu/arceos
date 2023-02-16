#![cfg_attr(not(test), no_std)]

extern crate alloc;

use alloc::boxed::Box;
use alloc::vec::Vec;
use alloc::string::String;

pub enum SeekFrom {
    Start(usize),
    Current(isize),
    End(isize)
}

// 文件读写操作
pub trait VfsFile {
    fn open(&self, path: &str) -> Option<Box<dyn VfsFile>>;
    fn mkdir(&self, folder_name: &str) -> Option<Box<dyn VfsFile>>;
    fn create(&self, file_name: &str) -> Option<Box<dyn VfsFile>>;
    fn read_dir(&self) -> Vec<String>;
    fn read(&self, buf: &mut [u8]) -> usize;
    fn write(&self, data: &[u8]) -> usize;
    fn seek(&self, seek: SeekFrom) -> usize;
    fn is_dir(&self) -> bool;
    fn is_file(&self) -> bool;
    fn close(&self);
}

// 尽量给予比较长的生命周期
pub trait VfsFileSystem: Send + Sync {
    fn name(&'static self) -> &'static str;
    fn root(&'static self) -> Box<dyn VfsFile>;
}

// 硬盘读写操作
pub trait DiskOperation {
    fn read_block(index: usize, buf: &mut [u8]);
    fn write_block(index: usize, data: &[u8]);
}