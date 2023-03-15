use crate::mem::PhysAddr;
use crate::misc::start;

pub fn start_secondary_cpu(cpu_id: usize, entry: PhysAddr, args: PhysAddr) {
    start(cpu_id, entry.as_usize(), args.as_usize());
}
