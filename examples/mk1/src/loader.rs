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

/// User stack size (16 KB)
pub const USER_STACK_SIZE: usize = 16 * 1024;

// Architecture-specific user stack addresses
// Different architectures have different virtual address space layouts

#[cfg(target_arch = "riscv64")]
mod arch_config {
    /// User stack top address for RISC-V64 (Sv39: 39-bit virtual address)
    pub const USER_STACK_TOP: usize = 0x8_0000_0000;
}

#[cfg(target_arch = "x86_64")]
mod arch_config {
    /// User stack top address for x86_64 (canonical lower half)
    pub const USER_STACK_TOP: usize = 0x7fff_f000_0000;
}

#[cfg(target_arch = "aarch64")]
mod arch_config {
    /// User stack top address for AArch64 (TTBR0 region)
    pub const USER_STACK_TOP: usize = 0x8_0000_0000;
}

#[cfg(target_arch = "loongarch64")]
mod arch_config {
    /// User stack top address for LoongArch64
    pub const USER_STACK_TOP: usize = 0x8_0000_0000;
}

pub use arch_config::USER_STACK_TOP;

/// User stack bottom address
pub const USER_STACK_BOTTOM: usize = USER_STACK_TOP - USER_STACK_SIZE;

// ============================================================================
// Architecture-specific user program binaries
// Each architecture needs its own machine code for the hello world program
// ============================================================================

/// RISC-V64 user program
/// ```asm
/// _start:
///     li a0, 1              # fd = stdout
///     la a1, msg            # buf
///     li a2, 14             # count
///     li a7, 64             # SYS_write
///     ecall
///     li a0, 0              # exit_code
///     li a7, 93             # SYS_exit  
///     ecall
///     j .
/// msg: .ascii "Hello, world!\n"
/// ```
#[cfg(target_arch = "riscv64")]
const USER_PROGRAM: &[u8] = &[
    // _start: (at offset 0x00)
    0x05, 0x45,             // c.li a0, 1 (at 0x00)
    0x97, 0x05, 0x00, 0x00, // auipc a1, 0 (at 0x02)
    0x93, 0x85, 0xe5, 0x01, // addi a1, a1, 30 (at 0x06, offset = 0x20 - 0x02 = 0x1e = 30)
    0x39, 0x46,             // c.li a2, 14 (at 0x0a)
    0x93, 0x08, 0x00, 0x04, // addi a7, zero, 64 (at 0x0c)
    0x73, 0x00, 0x00, 0x00, // ecall (at 0x10)
    0x01, 0x45,             // c.li a0, 0 (at 0x14)
    0x93, 0x08, 0xd0, 0x05, // addi a7, zero, 93 (at 0x16)
    0x73, 0x00, 0x00, 0x00, // ecall (at 0x1a)
    0x01, 0xa0,             // c.j 0 (at 0x1e)
    // msg: "Hello, world!\n" (at 0x20)
    b'H', b'e', b'l', b'l', b'o', b',', b' ', b'w', b'o', b'r', b'l', b'd', b'!', b'\n',
];

/// x86_64 user program  
/// ```asm
/// _start:
///     mov rax, 1            # SYS_write
///     mov rdi, 1            # fd = stdout
///     lea rsi, [rip+msg]    # buf
///     mov rdx, 14           # count
///     syscall
///     mov rax, 60           # SYS_exit
///     xor rdi, rdi          # exit_code = 0
///     syscall
///     jmp .
/// msg: .ascii "Hello, world!\n"
/// ```
#[cfg(target_arch = "x86_64")]
const USER_PROGRAM: &[u8] = &[
    // _start: (at offset 0x00)
    // mov rax, 1 (SYS_write)
    0x48, 0xc7, 0xc0, 0x01, 0x00, 0x00, 0x00,  // mov rax, 1 (at 0x00)
    // mov rdi, 1 (stdout)
    0x48, 0xc7, 0xc7, 0x01, 0x00, 0x00, 0x00,  // mov rdi, 1 (at 0x07)
    // lea rsi, [rip+offset] - offset to msg from next instruction
    0x48, 0x8d, 0x35, 0x15, 0x00, 0x00, 0x00,  // lea rsi, [rip+0x15] (at 0x0e, msg at 0x2a, next=0x15, 0x2a-0x15=0x15)
    // mov rdx, 14 (length)
    0x48, 0xc7, 0xc2, 0x0e, 0x00, 0x00, 0x00,  // mov rdx, 14 (at 0x15)
    // syscall
    0x0f, 0x05,                                 // syscall (at 0x1c)
    // mov rax, 60 (SYS_exit)
    0x48, 0xc7, 0xc0, 0x3c, 0x00, 0x00, 0x00,  // mov rax, 60 (at 0x1e)
    // xor rdi, rdi (exit_code = 0)
    0x48, 0x31, 0xff,                          // xor rdi, rdi (at 0x25)
    // syscall
    0x0f, 0x05,                                 // syscall (at 0x28)
    // msg: "Hello, world!\n" (at 0x2a)
    b'H', b'e', b'l', b'l', b'o', b',', b' ', b'w', b'o', b'r', b'l', b'd', b'!', b'\n',
];

/// AArch64 user program
/// ```asm
/// _start:
///     mov x8, #64           # SYS_write
///     mov x0, #1            # fd = stdout
///     adr x1, msg           # buf
///     mov x2, #14           # count
///     svc #0
///     mov x8, #93           # SYS_exit
///     mov x0, #0            # exit_code = 0
///     svc #0
///     b .
/// msg: .ascii "Hello, world!\n"
/// ```
#[cfg(target_arch = "aarch64")]
const USER_PROGRAM: &[u8] = &[
    // _start: (at offset 0x00)
    // mov x8, #64 (SYS_write)
    0x08, 0x08, 0x80, 0xd2,  // mov x8, #64 (at 0x00)
    // mov x0, #1 (stdout)
    0x20, 0x00, 0x80, 0xd2,  // mov x0, #1 (at 0x04)
    // adr x1, msg (PC-relative address, msg at 0x24, current at 0x08, offset = 0x1c)
    0xe1, 0x00, 0x00, 0x10,  // adr x1, #0x1c (at 0x08)
    // mov x2, #14 (length)
    0xc2, 0x01, 0x80, 0xd2,  // mov x2, #14 (at 0x0c)
    // svc #0
    0x01, 0x00, 0x00, 0xd4,  // svc #0 (at 0x10)
    // mov x8, #93 (SYS_exit)
    0xa8, 0x0b, 0x80, 0xd2,  // mov x8, #93 (at 0x14)
    // mov x0, #0 (exit_code)
    0x00, 0x00, 0x80, 0xd2,  // mov x0, #0 (at 0x18)
    // svc #0
    0x01, 0x00, 0x00, 0xd4,  // svc #0 (at 0x1c)
    // b . (infinite loop)
    0x00, 0x00, 0x00, 0x14,  // b . (at 0x20)
    // msg: "Hello, world!\n" (at 0x24)
    b'H', b'e', b'l', b'l', b'o', b',', b' ', b'w', b'o', b'r', b'l', b'd', b'!', b'\n',
];

/// LoongArch64 user program
/// ```asm
/// _start:
///     ori $a7, $zero, 64    # SYS_write ($a7 = $r11)
///     ori $a0, $zero, 1     # fd = stdout ($a0 = $r4)
///     pcaddu12i $a1, 0      # $a1 = PC (aligned) ($a1 = $r5)
///     addi.d $a1, $a1, 32   # $a1 = msg address (offset 0x20 from pcaddu12i)
///     ori $a2, $zero, 14    # count ($a2 = $r6)
///     syscall 0
///     ori $a7, $zero, 93    # SYS_exit
///     ori $a0, $zero, 0     # exit_code = 0
///     syscall 0
///     b .
/// msg: .ascii "Hello, world!\n"
/// ```
///
/// LoongArch64 instruction encoding reference:
/// - ORI:       bits[31:22]=0000001110, bits[21:10]=ui12, bits[9:5]=rj, bits[4:0]=rd
/// - ADDI.D:    bits[31:22]=0000001011, bits[21:10]=si12, bits[9:5]=rj, bits[4:0]=rd
/// - PCADDU12I: bits[31:25]=0001110, bits[24:5]=si20, bits[4:0]=rd
/// - SYSCALL:   bits[31:15]=00000000001010110, bits[14:0]=code
/// - B:         bits[31:26]=010100, bits[25:10]=offs[15:0], bits[9:0]=offs[25:16]
#[cfg(target_arch = "loongarch64")]
const USER_PROGRAM: &[u8] = &[
    // _start: (at offset 0x00)
    // ori $r11, $r0, 64 (SYS_write): 0000001110 000001000000 00000 01011 = 0x0381000B
    0x0B, 0x00, 0x81, 0x03,  // ori $a7, $zero, 64 (at 0x00)
    // ori $r4, $r0, 1 (stdout): 0000001110 000000000001 00000 00100 = 0x03800404
    0x04, 0x04, 0x80, 0x03,  // ori $a0, $zero, 1 (at 0x04)
    // pcaddu12i $r5, 0: 0001110 00000000000000000000 00101 = 0x1C000005
    0x05, 0x00, 0x00, 0x1C,  // pcaddu12i $a1, 0 (at 0x08)
    // addi.d $r5, $r5, 32: 0000001011 000000100000 00101 00101 = 0x02C080A5
    // (msg at 0x28, pcaddu12i at 0x08, offset = 0x28 - 0x08 = 0x20 = 32)
    0xA5, 0x80, 0xC0, 0x02,  // addi.d $a1, $a1, 32 (at 0x0C)
    // ori $r6, $r0, 14 (length): 0000001110 000000001110 00000 00110 = 0x03803806
    0x06, 0x38, 0x80, 0x03,  // ori $a2, $zero, 14 (at 0x10)
    // syscall 0: 00000000001010110 000000000000000 = 0x002B0000
    0x00, 0x00, 0x2B, 0x00,  // syscall 0 (at 0x14)
    // ori $r11, $r0, 93 (SYS_exit): 0000001110 000001011101 00000 01011 = 0x0381740B
    0x0B, 0x74, 0x81, 0x03,  // ori $a7, $zero, 93 (at 0x18)
    // ori $r4, $r0, 0 (exit_code): 0000001110 000000000000 00000 00100 = 0x03800004
    0x04, 0x00, 0x80, 0x03,  // ori $a0, $zero, 0 (at 0x1C)
    // syscall 0
    0x00, 0x00, 0x2B, 0x00,  // syscall 0 (at 0x20)
    // b . (offset = 0): 010100 0000000000000000 0000000000 = 0x50000000
    0x00, 0x00, 0x00, 0x50,  // b . (at 0x24)
    // msg: "Hello, world!\n" (at 0x28)
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
    /// Different architectures have different page table structures:
    /// - RISC-V Sv39: 512 entries, entries 256-511 for kernel (high addresses)
    /// - x86_64: 512 entries at PML4, entries 256-511 for kernel  
    /// - AArch64: Uses separate TTBR0 (user) and TTBR1 (kernel), but we copy for compatibility
    /// - LoongArch64: Similar to RISC-V, 512 entries
    fn copy_kernel_mappings(user_pt: &mut PageTable) -> Result<(), &'static str> {
        let kernel_pt_root = kernel_page_table_root();
        let user_pt_root = user_pt.root_paddr();
        
        // Get kernel and user top-level page table as slices
        let kernel_table = phys_to_virt(kernel_pt_root).as_usize() as *const u64;
        let user_table = phys_to_virt(user_pt_root).as_usize() as *mut u64;
        
        // Copy the upper half of the page table (kernel space entries)
        // All supported architectures use 512 entries at the top level,
        // with entries 256-511 covering the kernel space (high addresses)
        #[cfg(any(
            target_arch = "riscv64",
            target_arch = "x86_64",
            target_arch = "aarch64",
            target_arch = "loongarch64"
        ))]
        unsafe {
            for i in 256..512 {
                let kernel_entry = kernel_table.add(i).read();
                user_table.add(i).write(kernel_entry);
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
