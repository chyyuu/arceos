#![no_std]
#![no_main]

#[macro_use]
extern crate libax;
extern crate alloc;

use core::sync::atomic::{AtomicUsize, Ordering};
use core::time::Duration;
use libax::sync::{Mutex, WaitQueue};
use libax::task;
use libax::task::set_affinity;
use libax::time::{set_timer_interrupt, Instant};

const NUM_DATA: usize = 20_000_000;
const NUM_TASKS: usize = 4;

static FINISHED_TASKS: AtomicUsize = AtomicUsize::new(0);

static MAIN_WQ: WaitQueue = WaitQueue::new();
static RESULTS: Mutex<[u64; NUM_TASKS]> = Mutex::new([0; NUM_TASKS]); // TODO: task join

fn sqrt(n: u64) -> u64 {
    let mut x = n;
    loop {
        if x * x <= n && (x + 1) * (x + 1) > n {
            return x;
        }
        x = (x + n / x) / 2;
    }
}

#[no_mangle]
fn main() {
    let expect: u64 = (0..NUM_DATA as u64).map(sqrt).sum();

    for i in 0..NUM_TASKS {
        task::spawn(move || {
            set_affinity(1 << i);
            set_timer_interrupt(false);
            info!("Task {} started.", i);
            let left = i * (NUM_DATA / NUM_TASKS);
            let right = (left + (NUM_DATA / NUM_TASKS)).min(NUM_DATA);
            println!(
                "part {}: {:?} [{}, {})",
                i,
                task::current().id(),
                left,
                right
            );
            let t0 = Instant::now();
            let r0 = (left as u64..right as u64).map(sqrt).sum();
            let d0 = Instant::now().duration_since(t0);
            RESULTS.lock()[i] = r0;
            for t in 0..10 {
                let t0 = Instant::now();
                let r: u64 = (left as u64..right as u64).map(sqrt).sum();
                let d1 = Instant::now().duration_since(t0);
                let d = if d0 < d1 { d1 - d0 } else { d0 - d1 };
                assert_eq!(r, r0);
                println!("Recalc {}'s time diff: {:?}/{:?}", t, d, d0);
            }
            info!("Task {} ended.", i);

            set_timer_interrupt(true);

            println!("part {}: {:?} finished", i, task::current().id());
            let n = FINISHED_TASKS.fetch_add(1, Ordering::Relaxed);
            if n == NUM_TASKS - 1 {
                MAIN_WQ.notify_one(true);
            }
        });
    }

    let timeout = MAIN_WQ.wait_timeout(Duration::from_millis(5000));
    println!("main task woken up! timeout={}", timeout);

    let actual = RESULTS.lock().iter().sum();
    println!("sum = {}", actual);
    assert_eq!(expect, actual);

    println!("Parallel summation tests run OK!");
}
