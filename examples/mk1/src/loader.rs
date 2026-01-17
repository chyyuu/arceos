//! User program loader for mk1 kernel
//!
//! This module handles loading the user program into user address space.

use alloc::alloc::{alloc, Layout};
use axhal::paging::{MappingFlags, PageTable};
use axhal::mem::{phys_to_virt, virt_to_phys, PAGE_SIZE_4K, PhysAddr, VirtAddr};
use memory_addr::MemoryAddr;
use axmm::kernel_page_table_root;

/// User program entry point address
pub const USER_ENTRY: usize = 0x1000;

/// User stack top address  
pub const USER_STACK_TOP: usize = 0x8_0000_0000;

/// User stack size (16 KB)
pub const USER_STACK_SIZE: usize = 16 * 1024;

/// User stack bottom address
pub const USER_STACK_BOTTOM: usize = USER_STACK_TOP - USER_STACK_SIZE;

/// The embedded user program binary (hello world)
///
/// This is the machine code for:
/// ```asm
/// _start:
///     # sys_write(1, msg, 14)
///     li a0, 1
///     la a1, msg
///     li a2, 14
///     li a7, 64
///     ecall
///     # sys_exit(0)
///     li a0, 0
///     li a7, 93
///     ecall
///     j .
/// msg:
///     .ascii "Hello, world!\n"
/// ```
///
/// Compiled for RISC-V64 at address 0x1000
const USER_PROGRAM: &[u8] = &[
    // _start: (at offset 0x00)
    // li a0, 1
    0x05, 0x45,             // c.li a0, 1 (at 0x00)
    // la a1, msg (auipc + addi for msg at offset 0x20 from auipc at 0x02)
    0x97, 0x05, 0x00, 0x00, // auipc a1, 0 (at 0x02)
    0x93, 0x85, 0xe5, 0x01, // addi a1, a1, 30 (at 0x06, offset = 0x20 - 0x02 = 0x1e = 30)
    // li a2, 14
    0x39, 0x46,             // c.li a2, 14 (at 0x0a)
    // li a7, 64 (SYS_write)
    0x93, 0x08, 0x00, 0x04, // addi a7, zero, 64 (at 0x0c)
    // ecall
    0x73, 0x00, 0x00, 0x00, // ecall (at 0x10)
    // li a0, 0
    0x01, 0x45,             // c.li a0, 0 (at 0x14)
    // li a7, 93 (SYS_exit)
    0x93, 0x08, 0xd0, 0x05, // addi a7, zero, 93 (at 0x16)
    // ecall
    0x73, 0x00, 0x00, 0x00, // ecall (at 0x1a)
    // j .  (infinite loop, should never reach)
    0x01, 0xa0,             // c.j 0 (at 0x1e)
    // msg: "Hello, world!\n" (at 0x20)
    b'H', b'e', b'l', b'l', b'o', b',', b' ', b'w', b'o', b'r', b'l', b'd', b'!', b'\n',
];

/// Represents a user address space
pub struct UserAddrSpace {
    /// The page table for this address space
    page_table: PageTable,
    /// Virtual address of the code page (for deallocation)
    _code_vaddr: usize,
    /// Virtual address of the stack pages (for deallocation)
    _stack_vaddr: usize,
}

impl Default for UserAddrSpace {
    fn default() -> Self {
        Self::new().expect("Failed to create default UserAddrSpace")
    }
}

impl UserAddrSpace {
    /// Creates a new user address space and loads the user program
    pub fn new() -> Result<Self, &'static str> {
        // Create a new page table
        let mut page_table = PageTable::try_new().map_err(|_| "Failed to create page table")?;

        // Copy kernel page table entries to user page table
        // This is necessary because RISC-V uses a single satp register for both
        // user and kernel address spaces.
        Self::copy_kernel_mappings(&mut page_table)?;

        // Allocate physical memory for the user program (one page is enough)
        let code_vaddr = Self::alloc_pages(1)?;
        let code_paddr = virt_to_phys(VirtAddr::from(code_vaddr));
        
        // Copy the user program to the allocated page
        unsafe {
            let dst = code_vaddr as *mut u8;
            core::ptr::copy_nonoverlapping(USER_PROGRAM.as_ptr(), dst, USER_PROGRAM.len());
            // Zero the rest of the page
            core::ptr::write_bytes(dst.add(USER_PROGRAM.len()), 0, PAGE_SIZE_4K - USER_PROGRAM.len());
        }

        // Map user code page (read + execute + user accessible)
        let user_code_vaddr = VirtAddr::from(USER_ENTRY).align_down_4k();
        let flags = MappingFlags::READ | MappingFlags::EXECUTE | MappingFlags::USER;
        page_table
            .map(user_code_vaddr, code_paddr, page_table_multiarch::PageSize::Size4K, flags)
            .map_err(|_| "Failed to map user code")?
            .ignore();

        // Allocate physical memory for user stack
        let stack_pages = USER_STACK_SIZE / PAGE_SIZE_4K;
        let stack_vaddr = Self::alloc_pages(stack_pages)?;
        let stack_paddr = virt_to_phys(VirtAddr::from(stack_vaddr));
        
        // Zero the stack
        unsafe {
            core::ptr::write_bytes(stack_vaddr as *mut u8, 0, USER_STACK_SIZE);
        }

        // Map user stack pages (read + write + user accessible)
        let stack_flags = MappingFlags::READ | MappingFlags::WRITE | MappingFlags::USER;
        for i in 0..stack_pages {
            let vaddr = VirtAddr::from(USER_STACK_BOTTOM + i * PAGE_SIZE_4K);
            let paddr = PhysAddr::from(stack_paddr.as_usize() + i * PAGE_SIZE_4K);
            page_table
                .map(vaddr, paddr, page_table_multiarch::PageSize::Size4K, stack_flags)
                .map_err(|_| "Failed to map user stack")?
                .ignore();
        }

        log::info!(
            "User address space created: code @ {:#x} -> {:#x}, stack @ {:#x}-{:#x}",
            USER_ENTRY, code_paddr, USER_STACK_BOTTOM, USER_STACK_TOP
        );

        Ok(Self {
            page_table,
            _code_vaddr: code_vaddr,
            _stack_vaddr: stack_vaddr,
        })
    }

    /// Copy kernel page table entries to user page table
    /// 
    /// In RISC-V Sv39, the page table has 512 entries at the top level.
    /// Kernel space uses the high addresses (starting from 0xffffffc000000000),
    /// which corresponds to the last few entries (256-511) in the top-level table.
    fn copy_kernel_mappings(user_pt: &mut PageTable) -> Result<(), &'static str> {
        let kernel_pt_root = kernel_page_table_root();
        let user_pt_root = user_pt.root_paddr();
        
        // Get kernel and user top-level page table as slices
        let kernel_l2_table = phys_to_virt(kernel_pt_root).as_usize() as *const u64;
        let user_l2_table = phys_to_virt(user_pt_root).as_usize() as *mut u64;
        
        // Copy the upper half of the page table (kernel space entries)
        // In Sv39, entries 256-511 cover the kernel space (0xffffffc000000000 and above)
        unsafe {
            for i in 256..512 {
                let kernel_entry = kernel_l2_table.add(i).read();
                user_l2_table.add(i).write(kernel_entry);
            }
        }
        
        log::debug!(
            "Copied kernel mappings from {:#x} to {:#x}",
            kernel_pt_root, user_pt_root
        );
        
        Ok(())
    }

    /// Returns the root physical address of the page table
    pub fn page_table_root(&self) -> PhysAddr {
        self.page_table.root_paddr()
    }

    /// Translates a user virtual address to a physical address
    pub fn translate(&self, vaddr: VirtAddr) -> Option<PhysAddr> {
        match self.page_table.query(vaddr) {
            Ok((paddr, _, _)) => Some(paddr),
            Err(_) => None,
        }
    }

    /// Allocates contiguous 4K-aligned pages using the global allocator
    fn alloc_pages(count: usize) -> Result<usize, &'static str> {
        let size = count * PAGE_SIZE_4K;
        let layout = Layout::from_size_align(size, PAGE_SIZE_4K)
            .map_err(|_| "Invalid layout")?;
        
        let ptr = unsafe { alloc(layout) };
        if ptr.is_null() {
            return Err("Failed to allocate memory");
        }
        
        Ok(ptr as usize)
    }
}

impl Drop for UserAddrSpace {
    fn drop(&mut self) {
        // TODO: properly deallocate pages
        log::debug!("Dropping user address space");
    }
}
