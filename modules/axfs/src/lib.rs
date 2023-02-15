#![no_std]

#[macro_use]
extern crate alloc;
#[macro_use]
extern crate axlog;

use alloc::{vec::Vec, boxed::Box};
use axdriver::block_devices;
use fatfs_shim::Fat32FileSystem;
use lazy_init::LazyInit;
use vfscore::{VfsFileSystem, DiskOperation};
use driver_block::BlockDriverOps;

pub struct FileSystemList(Vec<Box<dyn VfsFileSystem>>);

impl FileSystemList {
    pub(crate) const fn new() -> Self {
        Self(vec![])
    }

    pub(crate) fn add(&mut self, fs: Box<dyn VfsFileSystem>) {
        // info!(
        //     "Added new {} filesystem",
        //     fs.as_ref().name()
        // );
        self.0.push(fs);
    }

    pub fn first(&self) -> Option<&Box<dyn VfsFileSystem>> {
        self.0.first()
    }
}

static FILESTSTEMS: LazyInit<FileSystemList> = LazyInit::new();

pub fn init_filesystems() {
    info!("init filesystems");
    let mut fs_list = FileSystemList::new();

    fs_list.add(Box::new(Fat32FileSystem::<DiskOps>::new()));
    FILESTSTEMS.init_by(fs_list);
}

pub struct DiskOps;

impl DiskOperation for DiskOps {
    fn read_block(index: usize, buf: &mut [u8]) {
        block_devices().0.read_block(index, buf).expect("can't read block");
    }

    fn write_block(index: usize, data: &[u8]) {
        block_devices().0.write_block(index, data).expect("can't write block");
    }
}

pub fn filesystems() -> &'static FileSystemList {
    &FILESTSTEMS
}