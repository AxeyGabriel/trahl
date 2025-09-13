use std::sync::Arc;
use zeromq::{prelude::*};
use tracing::{info, error};
use tokio::sync::{RwLock, mpsc, broadcast};
use tokio::task::JoinHandle;
use tokio;
use std::collections::HashMap;

use crate::rpc::{
    zmq_helper,
    Message,
};
use super::peers::*;
use super::MasterCtx;

const CHANNEL_BUFFER_SIZE: usize = 64;

#[derive(Debug, Clone)]
pub enum SocketEvent {
    PeerConnected(PeerId, mpsc::Sender<RxCoreDriverMsg>),
    PeerDisconnected(PeerId),
}

pub struct SocketServer {
    peer_map: HashMap<PeerId, (mpsc::Sender<Message>, JoinHandle<()>, String)>,
    tx_event: broadcast::Sender<SocketEvent>,
    tx_to_core: mpsc::Sender<TxCoreDriverMsg>,
}

impl SocketServer {
    pub fn new(
        tx_to_core: mpsc::Sender<TxCoreDriverMsg>,
    ) -> Self {
        let (tx_event, _) = broadcast::channel(64);
        SocketServer {
            peer_map: HashMap::new(),
            tx_event,
            tx_to_core,
        }
    }

    pub fn subscribe_for_events(&self) -> broadcast::Receiver<SocketEvent> {
        self.tx_event.subscribe()
    }

    pub async fn run(&mut self, ctx: Arc<MasterCtx>) {
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
                                        let (
                                            tx_sock_to_peer,
                                            rx_sock_to_peer
                                        ) = mpsc::channel::<Message>(CHANNEL_BUFFER_SIZE);
                                        
                                        let (
                                            tx_core_to_peer,
                                            rx_core_to_peer
                                        ) = mpsc::channel::<Message>(CHANNEL_BUFFER_SIZE);
                                        
                                        let p = Peer::new(
                                            hm.clone(),
                                            peer_id.to_vec(),
                                            tx_peer_to_sock.clone(),
                                            rx_sock_to_peer,
                                            self.tx_to_core.clone(),
                                            rx_core_to_peer,
                                        );
                                        
                                        let ph = tokio::spawn(p.run());

                                        self.peer_map
                                            .insert(peer_id.to_vec(), (tx_sock_to_peer, ph, hm.identifier.clone()));

                                        let _ = self.tx_event.send(SocketEvent::PeerConnected(
                                            peer_id.to_vec(),
                                            tx_core_to_peer,
                                        ));
                                        
                                        info!("New peer connected: {}", hm.identifier);
                                    }
                                }
                                Message::Bye => {
                                    if let Some(val) = self.peer_map
                                        .get(&peer_id) {
                                        val.1.abort();

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
                        error!("Error sending message to peer: {}", e)
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
    }
}
