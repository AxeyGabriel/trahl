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
use crate::master::peers::TxCoreDriverMsg;
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

    let (tx, mut rx) = mpsc::channel::<TxCoreDriverMsg>(8);
    let rpc_server = Arc::new(Mutex::new(SocketServer::new(tx)));
    let rpc_server_clone = rpc_server.clone();
    
    let task_driver = async move {
        let rpc_server_clone = rpc_server_clone.lock().await;
        let mut events = rpc_server_clone
            .subscribe_for_events();
        drop(rpc_server_clone);
        loop {
            tokio::select!(
                msg = rx.recv() => {
                    if let Some(msg) = msg {
                        info!("task_driver: {:#?}", msg);
                    }
                },
                event = events.recv() => {
                    if let Ok(SocketEvent::PeerConnected(peer_id, tx)) = event {
                        tx.send(Message::Bye).await;
                    }
                }
            );
        }
    };
    
    let rpc_server_clone = rpc_server.clone();
    let ctx_clone = ctx.clone();
    let rpc_server_task = async move {
        tokio::spawn(async move {
            rpc_server_clone.lock().await.run(ctx_clone).await;
        })
    };

    let _ = tokio::join!(
        tokio::spawn(web_service(ctx.clone())),
        tokio::spawn(rpc_server_task),
        tokio::spawn(task_driver),
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
