//! System call handling for mk1 kernel
//!
//! This module implements the sys_write and sys_exit system calls.

use axhal::uspace::UserContext;
use axhal::console::write_bytes;
use axhal::mem::phys_to_virt;
use memory_addr::{MemoryAddr, VirtAddr, PAGE_SIZE_4K};

use crate::loader::UserAddrSpace;

// Linux system call numbers differ by architecture
// Reference: https://chromium.googlesource.com/chromiumos/docs/+/master/constants/syscalls.md

#[cfg(target_arch = "x86_64")]
mod syscall_num {
    pub const SYS_WRITE: usize = 1;
    pub const SYS_EXIT: usize = 60;
}

#[cfg(target_arch = "aarch64")]
mod syscall_num {
    pub const SYS_WRITE: usize = 64;
    pub const SYS_EXIT: usize = 93;
}

#[cfg(target_arch = "riscv64")]
mod syscall_num {
    pub const SYS_WRITE: usize = 64;
    pub const SYS_EXIT: usize = 93;
}

#[cfg(target_arch = "loongarch64")]
mod syscall_num {
    pub const SYS_WRITE: usize = 64;
    pub const SYS_EXIT: usize = 93;
}

use syscall_num::{SYS_WRITE, SYS_EXIT};

/// Standard file descriptors
const STDOUT: usize = 1;
const STDERR: usize = 2;

/// Handles a system call from user space
///
/// Returns `true` if the program should continue, `false` if it should exit
pub fn handle_syscall(uctx: &mut UserContext, user_aspace: &UserAddrSpace) -> bool {
    let sysno = uctx.sysno();
    let arg0 = uctx.arg0();
    let arg1 = uctx.arg1();
    let arg2 = uctx.arg2();

    log::debug!(
        "syscall: num={}, arg0={:#x}, arg1={:#x}, arg2={:#x}",
        sysno, arg0, arg1, arg2
    );

    match sysno {
        SYS_WRITE => {
            let ret = sys_write(arg0, arg1, arg2, user_aspace);
            uctx.set_retval(ret as usize);
            true
        }
        SYS_EXIT => {
            sys_exit(arg0 as i32);
            // sys_exit never returns, but we need to satisfy the type system
            #[allow(unreachable_code)]
            false
        }
        _ => {
            log::warn!("Unknown syscall: {}", sysno);
            uctx.set_retval((-1isize) as usize);
            true
        }
    }
}

/// sys_write - write to a file descriptor
///
/// ssize_t write(int fd, const void *buf, size_t count);
fn sys_write(fd: usize, buf_ptr: usize, count: usize, user_aspace: &UserAddrSpace) -> isize {
    // Only support stdout and stderr
    if fd != STDOUT && fd != STDERR {
        log::warn!("sys_write: unsupported fd {}", fd);
        return -1;
    }

    if count == 0 {
        return 0;
    }

    // Read from user space memory
    // We need to translate the user virtual address to physical address
    match read_user_buffer(buf_ptr, count, user_aspace) {
        Ok(data) => {
            write_bytes(&data);
            count as isize
        }
        Err(e) => {
            log::warn!("sys_write: failed to read user buffer: {}", e);
            -1
        }
    }
}

/// sys_exit - terminate the process
///
/// void exit(int status);
fn sys_exit(exit_code: i32) -> ! {
    log::info!("User program exited with code: {}", exit_code);
    axhal::power::system_off()
}

/// Read data from user space buffer
///
/// This function translates user virtual addresses to physical addresses
/// and copies the data to a kernel buffer.
fn read_user_buffer(
    user_vaddr: usize,
    len: usize,
    user_aspace: &UserAddrSpace,
) -> Result<alloc::vec::Vec<u8>, &'static str> {
    let mut result = alloc::vec::Vec::with_capacity(len);
    let mut remaining = len;
    let mut current_vaddr = user_vaddr;

    while remaining > 0 {
        let vaddr = VirtAddr::from(current_vaddr);
        let page_offset = vaddr.align_offset_4k();
        let bytes_in_page = (PAGE_SIZE_4K - page_offset).min(remaining);

        // Walk the user page table to find the physical address
        // Note: page_table.query() already includes the page offset in the returned paddr
        let paddr = user_aspace.translate(vaddr)
            .ok_or("Failed to translate user address")?;

        // Copy data from the physical address (accessible via kernel linear mapping)
        // paddr already includes the page offset, no need to add it again
        let kernel_vaddr = phys_to_virt(paddr);
        let src = unsafe { core::slice::from_raw_parts(kernel_vaddr.as_ptr(), bytes_in_page) };
        result.extend_from_slice(src);

        current_vaddr += bytes_in_page;
        remaining -= bytes_in_page;
    }

    Ok(result)
}
