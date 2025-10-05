mod web;
mod file_watcher;
mod socket_server;
mod peers;
mod job_manager;

use tracing::{error, info};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::Ordering;
use tokio;
use tokio::sync::{mpsc, watch};
use tokio::sync::watch::{Receiver, Sender};
use tokio::time::{sleep, Duration};
use std::sync::Arc;
use std::sync::RwLock as SyncRwLock;

use socket_server::SocketEvent;

use web::web_service;
use socket_server::SocketServer;
use job_manager::JobManager;
use crate::config::SystemConfig;
use crate::master::job_manager::JobContract;
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

    let (
        tx_manager,
        rx_manager
    ) = mpsc::channel::<TxManagerMsg>(8);
    
    let (
        tx_socketserver,
        rx_socketserver
    ) = mpsc::channel::<SocketEvent>(8);
    
    let (
        tx_jobs,
        rx_jobs
    ) = mpsc::channel::<JobContract>(8);
    
    let job_manager = JobManager::new(
        rx_manager,
        rx_socketserver,
        rx_jobs,
    );
    
    let socket_server = SocketServer::new(
        tx_manager,
        tx_socketserver,
    );

/*    let str_f = PathBuf::from("/home/axey/trahl/test-resources/red_320x240_h264_1s.mp4");
    let script_p = PathBuf::from("/home/axey/trahl/test-resources/test.lua");
    
    let str_f_2 = PathBuf::from("/home/axey/trahl/test-resources/red_320x240_h264_1s.mp4");
    let script_p_2 = PathBuf::from("/home/axey/trahl/test-resources/test2.lua");
    
    let str_f_3 = PathBuf::from("/home/axey/repos/trahl/test-resources/red_320x240_h264_1s.mp4");
    let script_p_3 = PathBuf::from("/home/axey/repos/trahl/test-resources/test_transcode.lua");
    */
    let str_f_4 = PathBuf::from("/home/axey/repos/trahl/test-resources/red_320x240_h264_big.mp4");
    let script_p_4 = PathBuf::from("/home/axey/repos/trahl/test-resources/test_transcode.lua");
    let hm_4 = HashMap::new();
    
//    _ = tx_jobs.send(JobContract::new(str_f, HashMap::new(), script_p)).await;
//    _ = tx_jobs.send(JobContract::new(str_f_2, HashMap::new(), script_p_2)).await;
//    _ = tx_jobs.send(JobContract::new(PathBuf::from("/home/axey/trahl"), str_f_3, PathBuf::from("/tmp/dstdir"), hm_4.clone(), script_p_3)).await;
    _ = tx_jobs.send(JobContract::new(PathBuf::from("/home/axey/repos/trahl"), str_f_4, PathBuf::from("/tmp/dstdir"), hm_4.clone(), script_p_4)).await;

    let _ = tokio::join!(
        tokio::spawn(web_service(ctx.clone())),
        tokio::spawn(socket_server.run(ctx.clone())),
        tokio::spawn(job_manager.run(ctx.clone())),
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
