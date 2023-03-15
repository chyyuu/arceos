#[cfg(feature = "smp")]
use crate::lcpu;

mod boot;
mod dtb;
mod generic_timer;
mod pl011;

pub mod console;
pub mod irq;
pub mod mem;
pub mod misc;

pub mod mp;

pub mod time {
    pub use super::generic_timer::*;
}

pub(crate) fn platform_init(_dtb: *const u8) {
    extern "C" {
        fn exception_vector_base();
    }
    crate::mem::clear_bss();
    crate::arch::set_exception_vector_base(exception_vector_base as usize);
    self::generic_timer::init();
    self::dtb::init(_dtb);
    self::irq::init();
    let addr = dtb::prop_u64("arm,pl011", "reg").unwrap() as usize;
    self::pl011::init(addr);
    let method = dtb::prop_str("arm,psci-1.0", "method")
        .unwrap_or(dtb::prop_str("arm,psci-0.2", "method").unwrap());
    misc::init(method);
    self::irq::init_percpu(0); // TODO
    #[cfg(feature = "smp")]
    {
        lcpu::lcpu_init();
        dtb::smp_init();
    }
}

#[cfg(feature = "smp")]
pub(crate) fn platform_init_secondary(_dtb: *const u8) {
    extern "C" {
        fn exception_vector_base();
    }
    crate::arch::set_exception_vector_base(exception_vector_base as usize);
    self::irq::init_percpu(crate::arch::cpu_id());
}
