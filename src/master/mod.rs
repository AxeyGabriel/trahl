mod web;
mod file_watcher;
mod rpc_server;
mod peers;

use tracing::{error, info};
use std::sync::atomic::Ordering;
use tokio;
use tokio::sync::watch;
use tokio::sync::RwLock as AsyncRwLock;
use tokio::sync::watch::{Receiver, Sender};
use tokio::time::{sleep, Duration};
use std::sync::Arc;
use std::sync::RwLock as SyncRwLock;

use web::web_service;
use rpc_server::rpc_server;
use crate::config::SystemConfig;
use crate::master::peers::PeerRegistry;
use crate::{CONFIG, S_TERMINATE, S_RELOAD};

pub struct MasterCtx {
    pub ch_terminate:   (Sender<bool>, Receiver<bool>),
    pub ch_reload:      (Sender<bool>, Receiver<bool>),
    pub config:         Arc<SyncRwLock<SystemConfig>>,
    pub peer_registry:  Arc<AsyncRwLock<PeerRegistry>>,
}


pub fn master_thread() {
    let rt = tokio::runtime::Builder::new_multi_thread()
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
    let ctx = Arc::new(MasterCtx {
        ch_terminate: watch::channel(false),
        ch_reload: watch::channel(false),
        config: CONFIG.get().expect("configuration not initialized").clone(),
        peer_registry: Arc::new(AsyncRwLock::new(PeerRegistry::default())),
    });

    let _ = tokio::join!(
        tokio::spawn(web_service(ctx.clone())),
        tokio::spawn(rpc_server(ctx.clone())),
        task_propagate_signals(ctx.clone()),
    );

    info!("Master terminated");
}

async fn task_propagate_signals(ctx: Arc<MasterCtx>) {
    loop {
        let s_term = S_TERMINATE
            .get()
            .unwrap()
            .load(Ordering::Relaxed);

        let s_hup = S_RELOAD
            .get()
            .unwrap()
            .load(Ordering::Relaxed);

        if s_term {
            info!("Received termination signal");
            let _ = ctx.ch_terminate.0.send(true);
            break;
        }

        if s_hup {
            info!("Received reload signal");
            if let Some(flag) = S_RELOAD.get() {
                flag.store(false, Ordering::Relaxed);
            }
        }

        sleep(Duration::from_millis(100)).await;
    }
}
