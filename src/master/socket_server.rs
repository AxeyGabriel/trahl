use std::sync::Arc;
use zeromq::{prelude::*};
use tracing::{info, error};
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tokio;
use std::collections::HashMap;

use crate::rpc::WorkerInfo;
use crate::rpc::{
    zmq_helper,
    Message,
};
use super::peers::*;
use super::MasterCtx;

const CHANNEL_BUFFER_SIZE: usize = 64;

#[derive(Debug, Clone)]
pub enum SocketEvent {
    PeerConnected(PeerId, mpsc::Sender<RxManagerMsg>, WorkerInfo),
    PeerDisconnected(PeerId),
}

pub struct SocketServer {
    peer_map: HashMap<PeerId, (mpsc::Sender<Message>, JoinHandle<()>, String)>,
    tx_event: mpsc::Sender<SocketEvent>,
    tx_to_manager: mpsc::Sender<TxManagerMsg>,
}

impl SocketServer {
    pub fn new(
        tx_to_manager: mpsc::Sender<TxManagerMsg>,
        tx_event: mpsc::Sender<SocketEvent>,
    ) -> Self {
        SocketServer {
            peer_map: HashMap::new(),
            tx_event,
            tx_to_manager,
        }
    }

    pub async fn run(mut self, ctx: Arc<MasterCtx>) {
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

        let (
            tx_peer_to_sock,
            mut rx_peer_to_sock
        ) = mpsc::channel::<(PeerId, Message)>(CHANNEL_BUFFER_SIZE);

        let mut ch_term = ctx.ch_terminate.1.clone();

        loop {
            tokio::select!(
                recv = zmq_helper::recv_msg(&mut router, true) => {
                    match recv {
                        Ok((peer_id, msg)) => {
                            let peer_id = peer_id.unwrap();
                            match msg {
                                Message::Hello(hm) => {
                                    if !self.peer_map.contains_key(&peer_id) {
                                        info!("New peer connected: {}", hm.identifier);
                                        
                                        let (
                                            tx_sock_to_peer,
                                            rx_sock_to_peer
                                        ) = mpsc::channel::<Message>(CHANNEL_BUFFER_SIZE);
                                        
                                        let (
                                            tx_manager_to_peer,
                                            rx_manager_to_peer
                                        ) = mpsc::channel::<Message>(CHANNEL_BUFFER_SIZE);
                                        
                                        let p = Peer::new(
                                            hm.clone(),
                                            peer_id.to_vec(),
                                            tx_peer_to_sock.clone(),
                                            rx_sock_to_peer,
                                            self.tx_to_manager.clone(),
                                            rx_manager_to_peer,
                                        );
                                        
                                        let ph = tokio::spawn(p.run());

                                        self.peer_map
                                            .insert(peer_id.to_vec(), (tx_sock_to_peer, ph, hm.identifier.clone()));

                                        let _ = self.tx_event.send(SocketEvent::PeerConnected(
                                            peer_id.to_vec(),
                                            tx_manager_to_peer,
                                            hm,
                                        )).await;
                                        let _ = tx_peer_to_sock.send((peer_id.to_owned(), Message::ack())).await;
                                    }
                                }
                                Message::Bye => {
                                    if let Some(val) = self.peer_map
                                        .remove(&peer_id) {
                                        val.1.abort();
                                        
                                        let _ = self.tx_event.send(SocketEvent::PeerDisconnected(
                                            peer_id.to_vec(),
                                        )).await;

                                        info!("Disconnected peer {}", val.2);
                                    }
                                },
                                _ => {
                                    if let Some(val) = self.peer_map
                                        .get(&peer_id) {
                                        let tx = &val.0;
                                        let _ = tx.send(msg.clone()).await;
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            error!("Error while receiving message: {}", e);
                        }
                    }
                },
                Some((peer_id, msg)) = rx_peer_to_sock.recv() => {
                    if let Err(e) = zmq_helper::send_msg(&mut router, Some(&peer_id), &msg).await {
                        error!("Disconnected failed peer: {}", e);

                        if let Some(val) = self.peer_map
                            .remove(&peer_id) {
                            val.1.abort();
                        }
                        let _ = self.tx_event.send(SocketEvent::PeerDisconnected(
                            peer_id.to_vec(),
                        )).await;
                    }

                    if msg == Message::Bye {
                        if let Some(val) = self.peer_map
                            .remove(&peer_id) {
                            val.1.abort();
                        }
                        let _ = self.tx_event.send(SocketEvent::PeerDisconnected(
                            peer_id.to_vec(),
                        )).await;
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

        for (peer_id, v) in self.peer_map.iter() {
            let ph = &v.1;
            let _ = self.tx_event.send(SocketEvent::PeerDisconnected(
                peer_id.to_vec(),
            )).await;

            //let _ = tx_peer_to_sock.send((peer_id.to_owned(), Message::Bye)).await;
            let _ = zmq_helper::send_msg(&mut router, Some(&peer_id), &Message::bye()).await;

            ph.abort();
        }
    }
}
