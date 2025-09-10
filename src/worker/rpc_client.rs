use std::sync::Arc;
use zeromq::prelude::*;
use zeromq::DealerSocket;
use tracing::{info, error};
use tokio::time::{sleep, Duration};

use crate::rpc::Message;
use crate::rpc::zmq_helper;
use super::WorkerCtx;

pub async fn rpc_client(ctx: Arc<WorkerCtx>) {
    let master_addr = format!("tcp://{}",
        &ctx.config
        .read()
        .unwrap()
        .worker
        .master_addr);

    let mut socket = DealerSocket::new();
    if let Err(e) = socket.connect(&master_addr).await {
        error!("Failed to connect to master at {}: {}", master_addr, e);
        let _ = ctx.ch_terminate.0.send(true);
        return;
    }

    info!("Connected to master at {}", master_addr);

    let ch_term = &ctx.ch_terminate.1;
    loop {
        tokio::select!(
            _ = sleep(Duration::from_secs(1)) => {
                let msg = Message::HelloAck;
                match zmq_helper::send_msg(
                        &mut socket,
                        None,
                        &msg).await {
                    Ok(()) => {
                        info!("sent message: {:#?}", msg);
                    }
                    Err(e) => {
                        error!("error while sending message: {}", e);
                    }
                }
            },
            _ = sleep(Duration::from_secs(1)) => {
                if let Ok(_) = ch_term.has_changed() {
                    if *ch_term.borrow() {
                        break;
                    }
                } 
            }
        );
    }
}
