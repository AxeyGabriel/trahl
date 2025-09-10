use std::{sync::Arc, time::Instant};
use zeromq::{prelude::*};
use tracing::{info, error, debug};
use tokio::time::{sleep, Duration};

use crate::{master::peers::PeerInfo, rpc::{zmq_helper::{self, send_msg}, HelloMsg, Message}};
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
                    Ok((client_id, msg)) => {
                        rx_handler(ctx.clone(), &client_id.unwrap(), &msg).await;
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

    info!("Stopping orchestration service");

    let peers = &ctx.peer_registry
        .read()
        .await
        .peers;

    for (client_id, peer_info) in peers {
        let msg = Message::Bye;
        if let Err(e) = send_msg(&mut router, Some(client_id), &msg).await {
            error!("Error sending BYE to peer \"{}\": {}", peer_info.identifier, e);
        } else {
            debug!("Sent BYE to peer \"{}\"", peer_info.identifier);
        }
    }

}

async fn rx_handler(ctx: Arc<MasterCtx>, client_id: &[u8], msg: &Message) {
    info!("Received message: {:#?} {:#?}", client_id, msg);
    match msg {
        Message::Hello(m) => {
            if !ctx.peer_registry
                .read()
                .await
                .peers
                .contains_key(client_id) {
                let p = PeerInfo {
                    identifier: m.identifier.clone(),
                    simultaneous_jobs: m.simultaneous_jobs,
                    last_seen: Instant::now(),
                };
                info!("New worker discovered: {:#?}", p);
                ctx.peer_registry
                    .write()
                    .await
                    .peers
                    .insert(client_id.to_owned(), p);
            }
        }
        _ => {},
    }
}
