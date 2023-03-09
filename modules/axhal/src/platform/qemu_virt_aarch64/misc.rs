//! ARM Power State Coordination Interface.

use core::{arch::asm, panic};

use lazy_init::LazyInit;

static METHOD: LazyInit<fn(usize, usize, usize, usize) -> usize> = LazyInit::new();

const PSCI_CPU_ON: usize = 0x8400_0003;
const PSCI_SYSTEM_OFF: usize = 0x8400_0008;

fn psci_hvc_call(func: usize, arg0: usize, arg1: usize, arg2: usize) -> usize {
    let ret;
    unsafe {
        asm!(
            "hvc #0",
            inlateout("x0") func => ret,
            in("x1") arg0,
            in("x2") arg1,
            in("x3") arg2,
        )
    }
    ret
}

fn psci_smc_call(func: usize, arg0: usize, arg1: usize, arg2: usize) -> usize {
    let ret;
    unsafe {
        asm!(
            "smc #0",
            inlateout("x0") func => ret,
            in("x1") arg0,
            in("x2") arg1,
            in("x3") arg2,
        )
    }
    ret
}

pub fn init(method: &str) {
    match method {
        "hvc" => METHOD.init_by(psci_hvc_call),
        "smc" => METHOD.init_by(psci_smc_call),
        _ => panic!("Wrong PSCI method"),
    }
}

pub fn terminate() -> ! {
    info!("Shutting down...");
    METHOD(PSCI_SYSTEM_OFF, 0, 0, 0);
    unreachable!("It should shutdown!")
}

pub fn start(id: usize, entry: *const u8) {
    info!("Starting core {}...", id);
    assert_eq!(METHOD(PSCI_CPU_ON, id, entry as usize, 0), 0);
    info!("Started core {}!", id);
}
