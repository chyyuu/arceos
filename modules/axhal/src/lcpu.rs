use core::mem::transmute;
use core::sync::atomic::AtomicUsize;
use core::sync::atomic::Ordering::SeqCst;

use axconfig::SMP;
use lazy_init::LazyInit;
use memory_addr::PhysAddr;

use crate::{
    arch::wait_for_irqs, cpu::this_cpu_id as cpu_id, irq, platform::mp::start_secondary_cpu,
};
const OFFLINE: usize = 0;
const INIT: usize = OFFLINE + 1;
const IDLE: usize = INIT + 1;
const BUSY: usize = IDLE + 1;
struct LCPUState(AtomicUsize);
impl LCPUState {
    pub fn compare_exchange(&self, old: usize, new: usize) -> Result<usize, usize> {
        self.0.compare_exchange(old, new, SeqCst, SeqCst)
    }
    pub fn is_online(&self) -> bool {
        self.0.load(SeqCst) >= IDLE
    }
    pub fn is_busy(&self) -> bool {
        self.0.load(SeqCst) >= BUSY
    }
    pub fn add(&self, inc: usize) -> usize {
        self.0.fetch_add(inc, SeqCst)
    }
    pub fn sub(&self, dec: usize) -> usize {
        self.0.fetch_sub(dec, SeqCst)
    }
    pub fn store(&self, state: usize) {
        self.0.store(state, SeqCst)
    }
}
struct LCPU {
    pub id: usize,
    state: LCPUState,
    task: AtomicUsize,
}
impl LCPU {
    pub fn new(id: usize, state: usize) -> Self {
        Self {
            id,
            state: LCPUState(state.into()),
            task: 0.into(),
        }
    }
    pub fn start(&self, entry: PhysAddr, args: PhysAddr) {
        if self.state.compare_exchange(OFFLINE, INIT).is_ok() {
            start_secondary_cpu(self.id, entry, args);
        }
    }
    pub fn set_task(&self, task: fn()) {
        loop {
            if !self.state.is_online() {
                wait_for_irqs();
                continue;
            }
            self.state.add(1);
            if self
                .task
                .compare_exchange(0, task as usize, SeqCst, SeqCst)
                .is_err()
            {
                self.state.sub(1);
                wait_for_irqs();
                continue;
            }
            break;
        }
    }
    pub fn get_task(&self) -> fn() {
        let f = self.task.swap(0, SeqCst);
        if f == 0 {
            return || {};
        }
        unsafe { transmute(f) }
    }
    pub fn is_busy(&self) -> bool {
        self.state.is_busy()
    }
    pub fn finish_task(&self) {
        self.state.sub(1);
    }
    pub fn set_state(&self, state: usize) {
        self.state.store(state);
    }
}
struct LCPUManager {
    count: AtomicUsize,
    lcpus: [LazyInit<LCPU>; SMP],
}
const LCPUINIT: LazyInit<LCPU> = LazyInit::new();
impl LCPUManager {
    pub const fn new() -> Self {
        Self {
            count: AtomicUsize::new(0),
            lcpus: [LCPUINIT; SMP],
        }
    }
    pub fn init(&self) {
        self.add(cpu_id(), BUSY);
    }
    pub fn add(&self, id: usize, state: usize) {
        let idx = self.count.fetch_add(1, SeqCst);
        assert!(idx < SMP);
        self.lcpus[idx].init_by(LCPU::new(id, state));
    }
    pub fn get_idx(&self, id: usize) -> usize {
        let count = self.count.load(SeqCst);
        for idx in 0..count {
            if self.lcpus[idx].id == id {
                return idx;
            }
        }
        unreachable!("CPU not found");
    }
    pub fn get_lcpu(&self, idx: usize) -> &LCPU {
        assert!(idx < SMP);
        &self.lcpus[idx]
    }
    pub fn current_lcpu(&self) -> &LCPU {
        self.get_lcpu(self.get_idx(cpu_id()))
    }
}
static LCPUMANAGER: LCPUManager = LCPUManager::new();
pub fn lcpu_init() {
    LCPUMANAGER.init();
}
pub fn lcpu_add(id: usize) {
    LCPUMANAGER.add(id, OFFLINE);
}
pub fn lcpu_start(idx: usize, entry: PhysAddr, args: PhysAddr) {
    LCPUMANAGER.get_lcpu(idx).start(entry, args);
}
pub fn lcpu_started() {
    LCPUMANAGER.current_lcpu().set_state(IDLE);
}
//Todo: these irq numbers are just for aarch64
const RUN_IRQ: usize = 5;
const WAKE_IRQ: usize = 6;
pub fn lcpu_irq_init() {
    irq::register_handler(RUN_IRQ, || {
        let lcpu = LCPUMANAGER.current_lcpu();
        assert!(lcpu.is_busy());
        info!("Core {} receive a task", cpu_id());
        lcpu.get_task()();
        lcpu.finish_task();
    });
    irq::register_handler(WAKE_IRQ, || {
        info!("Core {} wake up", cpu_id());
    });
}
pub fn lcpu_run(idx: usize, task: fn()) {
    let lcpu = LCPUMANAGER.get_lcpu(idx);
    lcpu.set_task(task);
    irq::gen_sgi_to_cpu(RUN_IRQ as u32, lcpu.id);
}
pub fn lcpu_wakeup(idx: usize) {
    irq::gen_sgi_to_cpu(WAKE_IRQ as u32, LCPUMANAGER.get_lcpu(idx).id);
}
pub fn lcpu_wait(idx: usize) {
    while LCPUMANAGER.get_lcpu(idx).is_busy() {
        wait_for_irqs();
    }
}
