use lazy_init::LazyInit;
use memory_addr::PhysAddr;
use spinlock::SpinNoIrq;

use crate::{
    arch::{cpu_id, CPU_ID_MASK},
    irq,
    platform::mp::start_secondary_cpu,
};
pub const MAX_CORES: usize = 4;
enum LCPUState {
    OFFLINE,
    INIT,
    // IDLE,
    // BUSY,
}
struct LCPU {
    id: usize,
    state: LCPUState,
    task: fn(),
}
impl LCPU {
    pub fn new(id: usize, state: LCPUState) -> Self {
        Self {
            id,
            state,
            task: || {},
        }
    }
    pub fn start(&mut self, entry: PhysAddr, args: PhysAddr) {
        match self.state {
            LCPUState::OFFLINE => {
                start_secondary_cpu(self.id, entry, args);
                self.state = LCPUState::INIT;
            }
            _ => {}
        }
    }
    pub fn set_task(&mut self, task: fn()) {
        self.task = task.clone();
    }
    pub fn get_task(&mut self) -> fn() {
        let task = self.task;
        self.task = || {};
        task
    }
}
struct LCPUManager {
    count: usize,
    lcpus: [LCPU; MAX_CORES],
}
impl LCPUManager {
    pub fn new() -> Self {
        Self {
            count: 1,
            lcpus: [
                LCPU::new(cpu_id(), LCPUState::INIT),
                LCPU::new(CPU_ID_MASK, LCPUState::OFFLINE),
                LCPU::new(CPU_ID_MASK, LCPUState::OFFLINE),
                LCPU::new(CPU_ID_MASK, LCPUState::OFFLINE),
            ],
        }
    }
    pub fn add(&mut self, id: usize) {
        self.lcpus[self.count] = LCPU::new(id, LCPUState::OFFLINE);
        self.count += 1;
    }
    pub fn get_idx(&self, id: usize) -> usize {
        for idx in 0..self.count {
            if self.lcpus[idx].id == id {
                return idx;
            }
        }
        self.count
    }
    pub fn start(&mut self, id: usize, entry: PhysAddr, arg: PhysAddr) {
        let idx = self.get_idx(id);
        if idx < self.count {
            self.lcpus[idx].start(entry, arg);
        }
    }
    pub fn set_task(&mut self, id: usize, task: fn()) {
        let idx = self.get_idx(id);
        if idx < self.count {
            self.lcpus[idx].set_task(task);
        }
    }
    pub fn get_task(&mut self, id: usize) -> fn() {
        let idx = self.get_idx(id);
        if idx < self.count {
            return self.lcpus[idx].get_task();
        }
        return || {};
    }
}
static LCPUMANAGER: LazyInit<SpinNoIrq<LCPUManager>> = LazyInit::new();
pub fn lcpu_init() {
    LCPUMANAGER.init_by(SpinNoIrq::new(LCPUManager::new()));
}
pub fn add_lcpu(id: usize) {
    LCPUMANAGER.lock().add(id);
}
pub fn start(id: usize, entry: PhysAddr, args: PhysAddr) {
    LCPUMANAGER.lock().start(id, entry, args);
}
fn set_task(id: usize, task: fn()) {
    LCPUMANAGER.lock().set_task(id, task);
}
fn get_task() -> fn() {
    LCPUMANAGER.lock().get_task(cpu_id())
}
pub fn irq_init(run_irq: usize, wake_irq: usize) {
    irq::register_handler(run_irq, || {
        info!("Core {} receive a task", cpu_id());
        get_task()();
    });
    irq::register_handler(wake_irq, || {});
}
pub fn send_task(run_irq: usize, id: usize, task: fn()) {
    set_task(id, task);
    irq::gen_sgi_to_cpu(run_irq as u32, id);
}
