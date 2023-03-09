mod boot;
mod dtb;
mod generic_timer;
pub mod lcpu;
mod pl011;

pub mod console;
pub mod mem;
pub mod misc;

pub mod time {
    pub use super::generic_timer::{current_ticks, ticks_to_nanos};
}

pub fn platform_init(_dtb: *const u8) {
    extern "C" {
        fn exception_vector_base();
    }
    crate::mem::clear_bss();
    crate::arch::set_exception_vector_base(exception_vector_base as usize);
    generic_timer::init();
    dtb::init(_dtb);
    let addr = dtb::prop_u64("arm,pl011", "reg").unwrap() as usize;
    pl011::init(addr);
    let method = dtb::prop_str("arm,psci-1.0", "method")
        .unwrap_or(dtb::prop_str("arm,psci-0.2", "method").unwrap());
    misc::init(method);
    lcpu::lcpu_init();
    dtb::smp_init();
}
