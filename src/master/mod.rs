mod web;

use tracing::{error, info};
use std::sync::atomic::Ordering;
use tokio;
use tokio::sync::watch;
use tokio::time::{sleep, Duration};

use crate::master::web::web_service;
use crate::S_TERMINATE;

pub fn master_thread() {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build();

    match rt {
        Ok(rt) => {
            rt.block_on(master_runtime());
        }
        Err(e) => {
            error!("Failed to build tokio runtime: {}", e)
        }
    }
}

async fn master_runtime() {
    let (ch_term_tx, ch_term_rx) = watch::channel(false);

    let _ = tokio::join!(
        tokio::spawn(web_service(ch_term_rx.clone())),
        task_send_sigterm(ch_term_tx.clone()),
    );
}

async fn task_send_sigterm(tx: watch::Sender<bool>) {
    loop {
        let s_term = S_TERMINATE
            .get()
            .unwrap()
            .load(Ordering::Relaxed);

        if s_term {
            info!("Received termination signal");
            let _ = tx.send(true);
            break;
        }

        sleep(Duration::from_millis(100)).await;
    }
}
