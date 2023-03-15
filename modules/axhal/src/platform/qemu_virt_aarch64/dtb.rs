use fdt_rs::{
    base::{DevTree, DevTreeNode, DevTreeProp},
    prelude::{FallibleIterator, PropReader},
};
use lazy_init::LazyInit;

use crate::{
    mem::phys_to_virt,
    platform::qemu_virt_aarch64::lcpu::{add_lcpu, cpu_id},
};
static TREE: LazyInit<DevTree> = LazyInit::new();
pub(crate) fn init(_dtb: *const u8) {
    TREE.init_by(
        unsafe { DevTree::from_raw_pointer(phys_to_virt((_dtb as usize).into()).as_ptr()) }
            .unwrap(),
    );
}
fn compatible_node(compatible: &str) -> Option<DevTreeNode> {
    TREE.compatible_nodes(compatible).next().unwrap()
}
fn get_prop<'a>(node: &'a DevTreeNode<'a, 'static>, name: &str) -> DevTreeProp<'a, 'static> {
    node.props()
        .filter(|p| p.name().map(|s| s == name))
        .next()
        .unwrap()
        .unwrap()
}
pub fn prop_u64(compatible: &'static str, name: &str) -> Option<u64> {
    compatible_node(compatible).map(|n| get_prop(&n, name).u64(0).unwrap())
}
pub fn prop_str(compatible: &'static str, name: &str) -> Option<&'static str> {
    compatible_node(compatible).map(|n| get_prop(&n, name).str().unwrap())
}
fn get_node(name: &str) -> Option<DevTreeNode> {
    TREE.nodes()
        .filter(|n| n.name().map(|s| s == name))
        .next()
        .unwrap()
}
macro_rules! devices {
    ($device_type:expr) => {
        TREE.nodes().filter(|n| {
            n.props()
                .filter(|p| p.name().map(|s| s == "device_type"))
                .next()
                .map(|o| o.map_or(false, |p| p.str().unwrap() == $device_type))
        })
    };
}
pub fn smp_init() {
    let a = get_node("cpus")
        .map(|n| get_prop(&n, "#address-cells").u32(0).unwrap())
        .unwrap_or(2);
    assert!(a == 1 || a == 2);
    let bsp_id = cpu_id();
    let mut iter = devices!("cpu");
    loop {
        match iter.next().unwrap() {
            Some(n) => {
                let p = get_prop(&n, "reg");
                let id = if a == 1 {
                    p.u32(0).unwrap() as u64
                } else {
                    p.u64(0).unwrap()
                };
                if id != bsp_id {
                    add_lcpu(id);
                    assert_eq!(get_prop(&n, "enable-method").str(), Ok("psci"));
                }
            }
            None => {
                break;
            }
        }
    }
}
