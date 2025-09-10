mod jobrunner;
mod rpc_client;

use tracing::{error, info};
use std::sync::atomic::Ordering;
use tokio;
use tokio::sync::watch;
use tokio::sync::watch::{Receiver, Sender};
use tokio::time::{sleep, Duration};
use std::sync::{Arc, RwLock};

use crate::config::SystemConfig;
use crate::{CONFIG, S_TERMINATE, S_RELOAD};
use jobrunner::{JobRunner, JobSpec};
use rpc_client::rpc_client;

pub struct WorkerCtx {
    pub ch_terminate: (Sender<bool>, Receiver<bool>),
    pub ch_reload: (Sender<bool>, Receiver<bool>),
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
        ch_reload: watch::channel(false),
        config: CONFIG.get().expect("configuration not initialized").clone(),
    });

    let (job_runner, _hjs) = JobRunner::new().run();

    let mut jh = job_runner
        .spawn_job(JobSpec {
            vars: None,
            script: String::from(r#"
                _trahl.log(_trahl.INFO, "hello from lua")
                _trahl.log(_trahl.INFO, "hello from lua")
                _trahl.log(_trahl.INFO, "hello from lua")
                _trahl.log(_trahl.INFO, "hello from lua")
                _trahl.log(_trahl.INFO, "hello from lua")
                _trahl.log(_trahl.INFO, "hello from lua")
                _trahl.log(_trahl.INFO, "hello from lua")
                error
            "#),
        })
        .await;

    tokio::spawn(async move {
        while let Some(line) = jh.stdout_stream().recv().await {
            info!("Received: {}", line);
        }
    });

    //let result = jh.await_result().await;
    //info!("Job finished: {:?}", result);

    let _ = tokio::join!(
        task_propagate_signals(ctx.clone()),
        tokio::spawn(rpc_client(ctx.clone())),
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
