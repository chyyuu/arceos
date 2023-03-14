use crate::mem::virt_to_phys;
use crate::misc;
use crate::platform::qemu_virt_aarch64::boot::_start_secondary;
use aarch64_cpu::registers::MPIDR_EL1;
use lazy_init::LazyInit;
use spin::Mutex;
use tock_registers::interfaces::Readable;
const CPU_ID_MASK: u64 = 0xff00ffffff;
pub const MAX_CORES: usize = 4;
pub fn cpu_id() -> u64 {
    MPIDR_EL1.get() & CPU_ID_MASK
}
enum LCPUState {
    OFFLINE,
    INIT,
    IDLE,
    BUSY,
}
struct LCPU {
    id: u64,
    idx: usize,
    state: LCPUState,
}
impl LCPU {
    pub fn new(id: u64, idx: usize, state: LCPUState) -> Self {
        Self { id, idx, state }
    }
    pub fn start(&mut self) {
        match self.state {
            LCPUState::OFFLINE => {
                misc::start(
                    self.id as usize,
                    usize::from(virt_to_phys((_start_secondary as usize).into())) as *const u8,
                );
                self.state = LCPUState::INIT;
            }
            _ => {}
        }
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
                LCPU::new(cpu_id(), 0, LCPUState::INIT),
                LCPU::new(CPU_ID_MASK, 1, LCPUState::OFFLINE),
                LCPU::new(CPU_ID_MASK, 2, LCPUState::OFFLINE),
                LCPU::new(CPU_ID_MASK, 3, LCPUState::OFFLINE),
            ],
        }
    }
    pub fn add(&mut self, id: u64) {
        self.lcpus[self.count] = LCPU::new(id, self.count, LCPUState::OFFLINE);
        self.count += 1;
    }
    pub fn start(&mut self, idx: usize) {
        if idx < self.count {
            self.lcpus[idx].start();
        }
    }
}
static LCPUMANAGER: LazyInit<Mutex<LCPUManager>> = LazyInit::new();
pub fn lcpu_init() {
    LCPUMANAGER.init_by(Mutex::new(LCPUManager::new()));
}
pub fn add_lcpu(id: u64) {
    LCPUMANAGER.lock().add(id);
}
pub fn start(idx: usize) {
    LCPUMANAGER.lock().start(idx);
}
