use tracing::info;
use std::sync::atomic::Ordering;

use super::S_TERMINATE;

pub fn master_thread() {
    master_init();

    loop {
        let term = S_TERMINATE.get().unwrap();
        if term.load(Ordering::Relaxed) {
            break;
        }

        std::thread::sleep(std::time::Duration::from_secs(1));
    }
    
    master_exit();
}

fn master_init() {
    info!("master is initializing...");
}

fn master_exit() {
    info!("exiting");
}
