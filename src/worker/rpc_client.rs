use std::sync::Arc;
use zeromq::prelude::*;
use zeromq::DealerSocket;
use tracing::{info, error};
use tokio::time::{sleep, Duration};

use crate::rpc::HelloMsg;
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

    let msg = Message::Hello(HelloMsg {
        identifier: "abc".to_string(),
        simultaneous_jobs: 2,
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
                        rx_handler(ctx.clone(), &mut socket, &msg).await;
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


    info!("Disconnected");
    let msg = Message::Bye;
    let _ = zmq_helper::send_msg(
            &mut socket,
            None,
            &msg).await;
}

async fn rx_handler(ctx: Arc<WorkerCtx>, socket: &mut DealerSocket, msg: &Message) {
    match msg {
        Message::Bye => {
            info!("BYE received from master");
        },
        Message::HelloAck => {
            info!("Successfuly connected to master");
        },
        Message::Ping => {
            zmq_helper::send_msg(socket, None, &Message::Pong).await;
        },
        _ => {
            info!("Unknown message received: {:#?}", msg);
        },
    }
}
