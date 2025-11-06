mod db;
mod web;
mod librarian;
mod socket_server;
mod peers;
mod manager;

use tracing::{error, info};
use std::sync::atomic::Ordering;
use tokio;
use tokio::sync::{broadcast, mpsc, watch};
use tokio::sync::watch::{Receiver, Sender};
use tokio::time::{sleep, Duration};
use std::sync::Arc;
use std::sync::RwLock as SyncRwLock;

use socket_server::SocketEvent;

use web::web_service;
use socket_server::SocketServer;
use manager::JobManager;
use librarian::Librarian;
use crate::config::SystemConfig;
use crate::master::manager::{JobContract, events::ManagerEvent};
use crate::master::peers::TxManagerMsg;
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

    let dbpath = {
        ctx.config
            .read()
            .unwrap()
            .master.db_path
            .clone()
    };
    db::init_db(dbpath).await;
    db::merge_libs_config(&ctx.config.read().unwrap().jobs).await;
    
    let (
        tx_fullscan,
        rx_fullscan
    ) = mpsc::channel::<i64>(8);

    let (
        tx_events,
        _
    ) = broadcast::channel::<ManagerEvent>(256);

    let (
        tx_manager,
        rx_manager
    ) = mpsc::channel::<TxManagerMsg>(8);
    
    let (
        tx_socketserver,
        rx_socketserver
    ) = mpsc::channel::<SocketEvent>(8);
    
    let librarian = Librarian::new(rx_fullscan);
    
    let manager = JobManager::new(
        rx_manager,
        rx_socketserver,
        tx_events.clone(),
    );
    
    let socket_server = SocketServer::new(
        tx_manager,
        tx_socketserver,
    );

    tokio::spawn(web_service(ctx.clone(), tx_events));

    //let _ = tx_fullscan.send(1).await;

    let _ = tokio::join!(
        socket_server.run(ctx.clone()),
        manager.run(ctx.clone()),
        tokio::spawn(librarian.run(ctx.clone())),
        job_propagate_signals(ctx.clone()),
    );

    info!("Master terminated");
}

async fn job_propagate_signals(ctx: Arc<MasterCtx>) {
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
