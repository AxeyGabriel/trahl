mod jobrunner;
mod rpc_client;

use tracing::{error, info};
use std::sync::atomic::Ordering;
use tokio;
use tokio::sync::{mpsc, watch};
use tokio::sync::watch::{Receiver, Sender};
use tokio::time::{sleep, Duration};
use std::sync::{Arc, RwLock};

use crate::config::SystemConfig;
use crate::rpc::{JobStatusMsg, Message};
use crate::{CONFIG, S_TERMINATE, S_RELOAD};
use jobrunner::JobRunner;
use rpc_client::rpc_client;

pub struct WorkerCtx {
    pub ch_terminate: (Sender<bool>, Receiver<bool>),
    pub _ch_reload: (Sender<bool>, Receiver<bool>),
    pub config: Arc<RwLock<SystemConfig>>,
}

pub fn worker_thread() {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build();

    match rt {
        Ok(rt) => {
            rt.block_on(worker_runtime());
        }
        Err(e) => {
            error!("Failed to build tokio runtime: {}", e)
        }
    }
}

async fn worker_runtime() {
    let ctx = Arc::new(WorkerCtx {
        ch_terminate: watch::channel(false),
        _ch_reload: watch::channel(false),
        config: CONFIG.get().expect("configuration not initialized").clone(),
    });

    let (
        tx_to_socket,
        rx_to_socket
    ) = mpsc::channel::<Message>(8);
    
    let (
        tx_from_socket,
        mut rx_from_socket
    ) = mpsc::channel::<Message>(8);

    let (
        tx_from_job,
        mut rx_from_job
    ) = mpsc::channel::<JobStatusMsg>(8);

    let ctx_clone = ctx.clone();
    let cache_dir = {
        let cfg = ctx_clone.config.read().unwrap();
        cfg.worker.cache_dir.clone()
    };

    let remaps = {
        let cfg = ctx_clone.config.read().unwrap();
        cfg.worker.fs_remaps.clone()
    };

    let (job_runner, _jrh) = JobRunner::new(cache_dir, remaps).run();

    let ctx_clone = ctx.clone();
    let manager = async move {
        let mut ch_term = ctx_clone.ch_terminate.1.clone();

        loop {
            tokio::select!(
                Some(msg) = rx_from_socket.recv() => {
                    match msg {
                        Message::Bye => {
                            info!("BYE received from master");
                        },
                        Message::HelloAck => {
                            info!("Successfuly connected to master");
                        },
                        Message::Ping => {
                            _ = tx_to_socket.send(Message::pong()).await;
                        },
                        Message::Job(jobmsg) => {
                            info!("Job received: {}", jobmsg.job_id);
                            job_runner.spawn_job(jobmsg, tx_from_job.clone()).await;
                        },
                        _ => {
                            info!("Unknown message received: {:#?}", msg);
                        },
                    }
                },
                Some(msg) = rx_from_job.recv() => {
                    let msg = Message::job_status(msg);
                    _ = tx_to_socket.send(msg).await;
                },
                _ = ch_term.changed() => {
                    if *ch_term.borrow() {
                        break;
                    }
                }
            )
        }
    };

    let _ = tokio::join!(
        task_propagate_signals(ctx.clone()),
        tokio::spawn(rpc_client(ctx.clone(), rx_to_socket, tx_from_socket.clone())),
        manager
    );
}

async fn task_propagate_signals(ctx: Arc<WorkerCtx>) {
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
