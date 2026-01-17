//! mk1 - A Minimal Monolithic Kernel
//!
//! This is a minimal monolithic kernel example that demonstrates:
//! - User/kernel space separation
//! - System call handling (sys_write and sys_exit)
//! - Running a simple user program that prints "Hello, world!"
//!
//! # Architecture
//!
//! ```text
//! +-------------------+
//! |    User Space     |
//! |  (hello program)  |
//! +-------------------+
//!         | ecall
//!         v
//! +-------------------+
//! |   Kernel Space    |
//! |   (mk1 kernel)    |
//! +-------------------+
//! ```

#![cfg_attr(feature = "axstd", no_std)]
#![cfg_attr(feature = "axstd", no_main)]

extern crate alloc;

#[cfg(feature = "axstd")]
use axstd::println;

mod loader;
mod syscall;

use axhal::uspace::{UserContext, ReturnReason};
use loader::{UserAddrSpace, USER_ENTRY, USER_STACK_TOP};

/// The main entry point of the mk1 kernel
#[cfg_attr(feature = "axstd", unsafe(no_mangle))]
fn main() {
    println!("=== mk1: Minimal Monolithic Kernel ===");
    println!("Starting mk1 kernel...");

    // Create user address space and load the user program
    println!("Creating user address space...");
    let user_aspace = match UserAddrSpace::new() {
        Ok(aspace) => aspace,
        Err(e) => {
            println!("Failed to create user address space: {}", e);
            return;
        }
    };

    let user_pt_root = user_aspace.page_table_root();
    println!("User page table root: {:#x}", user_pt_root);

    // Switch to user page table (which includes kernel mappings)
    println!("Switching to user page table...");
    unsafe {
        axhal::asm::write_user_page_table(user_pt_root);
        axhal::asm::flush_tlb(None);
    }
    println!("Page table switched successfully!");

    // Create user context
    println!("Creating user context...");
    println!("  Entry point: {:#x}", USER_ENTRY);
    println!("  Stack top: {:#x}", USER_STACK_TOP);
    
    let mut uctx = UserContext::new(
        USER_ENTRY,
        memory_addr::VirtAddr::from(USER_STACK_TOP),
        0,  // arg0 (not used)
    );

    println!("Entering user space...");
    println!("---");

    // Main loop: run user program and handle system calls
    loop {
        // Enter user space
        let reason = uctx.run();

        match reason {
            ReturnReason::Syscall => {
                // Handle system call
                if !syscall::handle_syscall(&mut uctx, &user_aspace) {
                    // User program wants to exit
                    break;
                }
            }
            ReturnReason::Interrupt => {
                // Interrupts are handled by the kernel automatically
                log::debug!("Interrupt occurred");
            }
            ReturnReason::PageFault(vaddr, flags) => {
                println!("Page fault at {:#x} with flags {:?}", vaddr, flags);
                println!("User context: {:?}", &*uctx);
                break;
            }
            ReturnReason::Exception(info) => {
                println!("Exception: {:?}", info);
                println!("User context: {:?}", &*uctx);
                break;
            }
            ReturnReason::Unknown => {
                println!("Unknown return reason");
                println!("User context: {:?}", &*uctx);
                break;
            }
        }
    }

    println!("---");
    println!("mk1 kernel exiting...");
}
