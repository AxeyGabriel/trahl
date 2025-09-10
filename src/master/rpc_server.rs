use std::sync::Arc;
use zeromq::{prelude::*};
use tracing::{info, error};
use tokio::time::{sleep, Duration};

use crate::rpc::zmq_helper;
use super::MasterCtx;

pub async fn rpc_server(ctx: Arc<MasterCtx>) {
    let bind_addr = format!("tcp://{}",
        &ctx.config
        .read()
        .unwrap()
        .master.orch_bind_addr);

    let mut router = zeromq::RouterSocket::new();
    if let Err(e) = router.bind(&bind_addr).await {
        error!("Orchestration failed to bind to {}: {}", bind_addr, e);
        let _ = ctx.ch_terminate.0.send(true);
        return;
    }

    info!("Orchestration service listening at {}", bind_addr);

    let mut ch_term = ctx.ch_terminate.1.clone();
    loop {
        tokio::select!(
            msg = zmq_helper::recv_msg(&mut router, true) => {
                match msg {
                    Ok((id, msg)) => {
                        info!("Received message: {:#?} {:#?}", id, msg);
                    }
                    Err(e) => {
                        error!("Error while receiving message: {}", e);
                    }
                }
            },
            _ = ch_term.changed() => {
                if *ch_term.borrow() {
                    break;
                }
            }
        );
    }
/*
    todo: send bye to all workers
*/
    sleep(Duration::from_secs(1)).await;
    info!("Stopping orchestration service");
    let _ = router.unbind_all();

}

