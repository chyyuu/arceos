use axconfig::{SMP, TASK_STACK_SIZE};
use axhal::{
    arch::wait_for_irqs,
    cpu::this_cpu_id as cpu_id,
    lcpu::{lcpu_run, lcpu_start, lcpu_started, lcpu_wait, lcpu_wakeup},
    mem::{virt_to_phys, VirtAddr},
};

use crate::{RUN_IRQ, WAKE_IRQ};

#[link_section = ".bss.stack"]
static mut SECONDARY_BOOT_STACK: [[u8; TASK_STACK_SIZE]; SMP - 1] = [[0; TASK_STACK_SIZE]; SMP - 1];

extern "C" {
    fn _start_secondary();
}

pub fn start_secondary_cpus(primary_cpu_id: usize) {
    lcpu_started();
    let entry = virt_to_phys(VirtAddr::from(_start_secondary as usize));
    let mut logic_cpu_id = 0;
    for i in 0..SMP {
        if i != primary_cpu_id {
            let stack_top = virt_to_phys(VirtAddr::from(unsafe {
                SECONDARY_BOOT_STACK[logic_cpu_id].as_ptr_range().end as usize
            }));

            debug!("starting CPU {}...", i);
            // todo: args should be address of args
            lcpu_start(i, entry, stack_top);
            logic_cpu_id += 1;
        }
        lcpu_run(RUN_IRQ, i, test_irq);
    }
    for i in 0..SMP {
        lcpu_wait(i);
    }
}

#[no_mangle]
pub extern "C" fn rust_main_secondary(cpu_id: usize) -> ! {
    info!("Secondary CPU {} started.", cpu_id);

    #[cfg(feature = "paging")]
    init_paging_secondary();

    info!("Secondary CPU {} init OK.", cpu_id);
    super::INITED_CPUS.fetch_add(1, core::sync::atomic::Ordering::Relaxed);

    while !super::is_init_ok() {
        core::hint::spin_loop();
    }

    axhal::arch::enable_irqs();
    lcpu_started();
    loop {
        wait_for_irqs(); // TODO
    }
}

fn test_irq() {
    if cpu_id() != 0 {
        info!("Secondary CPU {} run a task.", cpu_id());
        lcpu_wakeup(WAKE_IRQ, 0);
    } else {
        info!("Primary CPU {} run a task.", cpu_id());
    }
}

#[cfg(feature = "paging")]
fn init_paging_secondary() {}
