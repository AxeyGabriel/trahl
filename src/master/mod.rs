mod web;
mod file_watcher;
mod socket_server;
mod peers;

use tracing::{error, info};
use std::sync::atomic::Ordering;
use tokio;
use tokio::sync::{broadcast, mpsc, watch};
use tokio::sync::watch::{Receiver, Sender};
use tokio::time::{sleep, Duration};
use tokio::sync::Mutex;
use std::sync::Arc;
use std::sync::RwLock as SyncRwLock;

use socket_server::SocketEvent;

use web::web_service;
use socket_server::SocketServer;
use crate::config::SystemConfig;
use crate::master::peers::TxManagerMsg;
use crate::rpc::Message;
use crate::{CONFIG, S_TERMINATE, S_RELOAD};

pub struct MasterCtx {
    pub ch_terminate:   (Sender<bool>, Receiver<bool>),
    pub ch_reload:      (Sender<bool>, Receiver<bool>),
    pub config:         Arc<SyncRwLock<SystemConfig>>,
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
    });

    let (
        tx_manager,
        mut rx_manager
    ) = mpsc::channel::<TxManagerMsg>(8);
    
    let (
        tx_socketserver,
        mut rx_socketserver
    ) = mpsc::channel::<SocketEvent>(8);
    
    let task_manager = async move {
        loop {
            tokio::select!(
                msg = rx_manager.recv() => {
                    if let Some(msg) = msg {
                        info!("task_driver: {:#?}", msg);
                    }
                },
                event = rx_socketserver.recv() => {
                    if let Some(SocketEvent::PeerConnected(peer_id, tx)) = event {
                        info!("ev rx");
                        let _ = tx.send(Message::Bye).await;
                    }
                },
            );
        }
    };
    
    let socket_server = SocketServer::new(
        tx_manager,
        tx_socketserver,
    );

    let _ = tokio::join!(
        tokio::spawn(web_service(ctx.clone())),
        tokio::spawn(socket_server.run(ctx.clone())),
        tokio::spawn(task_manager),
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
