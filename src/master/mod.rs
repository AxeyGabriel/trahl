use tracing::info;
use std::sync::atomic::Ordering;
use std::cell::RefCell;
use tokio;

use super::{S_TERMINATE, CONFIG};

thread_local! {
    static CTX: RefCell<Context> = RefCell::default();
}

struct Context {
    /* ORCH socket
     * WEB socket
     * KnownWorkers
     * PendingJobs
     * WorkingJobs
     */
    exiting: bool,
}

impl Default for Context {
    fn default() -> Self {
        Context {
            exiting: false,
        }
    } 
}

pub fn master_thread() {
    let mut context = Context::default();

    let async_rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();

    async_rt.block_on(async { 
        master_init(&mut context).await;

        while !CTX.with_borrow(|c| c.exiting) {
            master_loop().await;
        }

        master_exit().await;
    });
}

async fn master_init(_ctx: &mut Context) {
    info!("master is initializing...");
}

async fn master_loop() {
    let term = S_TERMINATE.get().unwrap();
    if term.load(Ordering::Relaxed) {
        CTX.with_borrow_mut(|c| c.exiting = true);
    }

    std::thread::sleep(std::time::Duration::from_secs(1));
}

async fn master_exit() {
    info!("exiting");
}
