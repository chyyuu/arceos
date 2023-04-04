#[cfg(feature = "smp")]
use crate::lcpu;

mod boot;
mod dtb;
pub use dtb::cmdline;
mod generic_timer;
mod pl011;
mod psci;

pub mod console;
pub mod irq;
pub mod mem;

#[cfg(feature = "smp")]
pub mod mp;

pub mod time {
    pub use super::generic_timer::*;
}

pub mod misc {
    pub use super::psci::system_off as terminate;
}

extern "C" {
    fn exception_vector_base();
}

pub(crate) fn platform_init(cpu_id: usize, _dtb: *const u8) {
    crate::mem::clear_bss();
    crate::arch::set_exception_vector_base(exception_vector_base as usize);
    crate::cpu::init_percpu(cpu_id, true);
    self::generic_timer::init();
    self::dtb::init(_dtb);
    self::irq::init();
    let addr = dtb::prop_u64("arm,pl011", "reg").unwrap() as usize;
    self::pl011::init(addr);
    let method = dtb::prop_str("arm,psci-1.0", "method")
        .unwrap_or(dtb::prop_str("arm,psci-0.2", "method").unwrap());
    psci::init(method);
    self::irq::init_percpu(cpu_id);
    #[cfg(feature = "smp")]
    {
        lcpu::lcpu_init();
        dtb::smp_init();
    }
}

#[cfg(feature = "smp")]
pub(crate) fn platform_init_secondary(cpu_id: usize, _dtb: *const u8) {
    crate::arch::set_exception_vector_base(exception_vector_base as usize);
    crate::cpu::init_percpu(cpu_id, false);
    self::irq::init_percpu(cpu_id);
    self::generic_timer::init_secondary();
}
