use std::sync::Arc;
use tokio::sync::mpsc;
use zeromq::prelude::*;
use zeromq::DealerSocket;
use tracing::{info, error};

use crate::rpc::WorkerInfo;
use crate::rpc::Message;
use crate::rpc::zmq_helper;
use super::WorkerCtx;

pub async fn rpc_client(
    ctx: Arc<WorkerCtx>,
    mut rx: mpsc::Receiver::<Message>,
    tx: mpsc::Sender::<Message>,
) {
    let worker_config = {
        let cfg = &ctx.config
        .read()
        .unwrap();
        cfg.worker.clone()
    };

    let master_addr = format!("tcp://{}",
        worker_config.master_addr);

    let mut socket = DealerSocket::new();
    if let Err(e) = socket.connect(&master_addr).await {
        error!("Failed to connect to master at {}: {}", master_addr, e);
        let _ = ctx.ch_terminate.0.send(true);
        return;
    }

    info!("Connected to master at {}", master_addr);

    let msg = Message::hello(WorkerInfo {
        identifier: worker_config.identifier,
        simultaneous_jobs: worker_config.parallel_jobs,
        sw_version: env!("CARGO_PKG_VERSION_MAJOR").to_string(),
    });

    match zmq_helper::send_msg(
            &mut socket,
            None,
            &msg).await {
        Ok(()) => {
        }
        Err(e) => {
            error!("error while sending message: {}", e);
        }
    };

    let mut ch_term = ctx.ch_terminate.1.clone();
    loop {
        tokio::select!(
            msg = zmq_helper::recv_msg(&mut socket, false) => {
                match msg {
                    Ok((_, msg)) => {
                        _ = tx.send(msg).await;
                    }
                    Err(e) => {
                        error!("Error while receiving message: {}", e);
                    }
                }
            },
            Some(rxmsg) = rx.recv() => {
                _ = zmq_helper::send_msg(&mut socket, None, &rxmsg).await;
            }
            _ = ch_term.changed() => {
                if *ch_term.borrow() {
                    break;
                }
            }
        );
    }


    info!("Disconnected");
    let msg = Message::bye();
    let _ = zmq_helper::send_msg(
            &mut socket,
            None,
            &msg).await;
}
