use tracing::info;
use std::sync::atomic::Ordering;

use super::S_TERMINATE;

pub fn worker_thread() {
    worker_init();

    loop {
        let term = S_TERMINATE.get().unwrap();
        if term.load(Ordering::Relaxed) {
            break;
        }

        std::thread::sleep(std::time::Duration::from_secs(1));
    }
    
    worker_exit();
}

fn worker_init() {
    info!("worker is initializing...");
}

fn worker_exit() {
    info!("exiting");
}
