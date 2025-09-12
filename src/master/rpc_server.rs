use std::sync::Arc;
use zeromq::{prelude::*};
use tracing::{info, error, debug};
use tokio::time::{interval, Duration};
use tokio::sync::{RwLock, mpsc, broadcast};
use tokio::sync::Mutex;
use std::time::Instant;
use std::collections::HashMap;

use crate::rpc::{
    zmq_helper::{self, send_msg},
    Message,
};
use super::peers::{Peer, PeerId};
use super::MasterCtx;

const CHANNEL_BUFFER_SIZE: usize = 64;

#[derive(Debug, Clone)]
pub enum RPCEvent {
    PeerConnected(PeerId),
    PeerDisconnected { peer: PeerId, timedout: bool },
}

#[derive(Debug)]
pub struct PeerRegistry {
    peers: HashMap<Vec<u8>, (Peer, mpsc::Sender<Message>)>
}

impl Default for PeerRegistry {
    fn default() -> Self {
        PeerRegistry { peers: HashMap::new() }
    }
}

impl PeerRegistry {
    pub fn contains(&self, id: &[u8]) -> bool {
        self.peers.contains_key(id)
    }
    
    pub fn get(&self, id: &[u8]) -> Option<&(Peer, mpsc::Sender<Message>)> {
        self.peers
            .get(id)
    }
    
    pub fn iter(&self) -> impl Iterator<Item = (&[u8], &Peer)> {
        self.peers
            .iter()
            .map(|(k, (peer, _))| (k.as_slice(), peer))
    }
}

pub struct RpcServer {
    peer_registry: Arc<RwLock<PeerRegistry>>,
    event_tx: broadcast::Sender<RPCEvent>,
}

impl RpcServer {
    pub fn new() -> Self {
        let (event_tx, _) = broadcast::channel(64);
        RpcServer {
            peer_registry: Arc::new(RwLock::new(PeerRegistry::default())),
            event_tx,
        }
    }

    pub fn subscribe_for_events(&self) -> broadcast::Receiver<RPCEvent> {
        self.event_tx.subscribe()
    }

    pub async fn peer_registry(&self) -> Arc<RwLock<PeerRegistry>> {
        self.peer_registry.clone()
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

        let (tx_peer_to_sock, mut rx_peer_to_sock) = mpsc::channel::<(PeerId, Message)>(CHANNEL_BUFFER_SIZE);

        let mut keepalive_poll = interval(Duration::from_secs(2));
        let mut ch_term = ctx.ch_terminate.1.clone();

        loop {
            tokio::select!(
                msg = zmq_helper::recv_msg(&mut router, true) => {
                    match msg {
                        Ok((client_id, msg)) => {
                            self.rx_handler(&client_id.unwrap(), &msg, tx_peer_to_sock.clone()).await;
                        }
                        Err(e) => {
                            error!("Error while receiving message: {}", e);
                        }
                    }
                },
                Some((peer_id, msg)) = rx_peer_to_sock.recv() => {
                    if let Err(e) = send_msg(&mut router, Some(&peer_id), &msg).await {
                        error!("Error sending message to peer: {}", e)
                    }
                },
                _ = keepalive_poll.tick() => {
                    let mut to_remove: Vec<PeerId> = Vec::new();

                    for (client_id, peer) in self.peer_registry
                        .read()
                        .await
                        .peers
                        .iter() {
                        if let Err(e) = send_msg(&mut router, Some(client_id), &Message::Ping).await {
                            error!("Error sending message to peer: {}", e)
                        }
                    
                        let tx = &peer.1;
                        let peer = &peer.0;

                        if Instant::now().duration_since(*peer.get_last_seen()) > Duration::from_secs(5) {
                            let _ = self.event_tx
                                .send(RPCEvent::PeerDisconnected {peer: client_id.to_vec(), timedout: true});
                            info!("Peer \"{}\" timed out", peer.get_params().identifier);
                            to_remove.push(client_id.to_vec());

                            let _ = tx.send(Message::Bye).await;
                        }
                    }

                    for k in &to_remove {
                        self.peer_registry
                            .write()
                            .await
                            .peers
                            .remove(k);
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

        for (client_id, peer_info) in self.peer_registry
            .read()
            .await
            .peers
            .iter() {
            let msg = Message::Bye;
            let identifier = &peer_info.0.get_params().identifier;
            if let Err(e) = send_msg(&mut router, Some(client_id), &msg).await {
                error!("Error sending BYE to peer \"{}\": {}", identifier, e);
            }
        }
    }

    async fn rx_handler(
        &mut self,
        client_id: &[u8],
        msg: &Message,
        tx: mpsc::Sender<(PeerId, Message)>,
    ) {
        match msg {
            Message::Hello(m) => {
                if !self.peer_registry
                    .read()
                    .await
                    .peers
                    .contains_key(client_id) {
        
                    let (tx_sock_to_peer, rx_sock_to_peer) = mpsc::channel::<Message>(CHANNEL_BUFFER_SIZE);
                    
                    let p = Peer::new(m.clone(),
                        client_id.to_vec(),
                        tx,
                        rx_sock_to_peer,
                    );
                    
                    info!("New worker discovered: {}", p.get_params().identifier);
                    
                    self.peer_registry
                        .write()
                        .await
                        .peers
                        .insert(client_id.to_vec(), (p, tx_sock_to_peer));
        
                    let _ = self.event_tx
                        .send(RPCEvent::PeerConnected(client_id.to_vec()));
                }
            },
            Message::Pong => {
                match self.peer_registry
                    .write()
                    .await
                    .peers
                    .get_mut(client_id) {
                    Some(peer) => {
                        peer.0.update_last_seen();
                    },
                    None => {
                        error!("Received message from unknown peer");
                    }
                }
            },
            _ => {},
        }

        match self.peer_registry
            .read()
            .await
            .get(client_id) {
            Some(peer) => {
                let tx = &peer.1;
                let _ = tx.send(msg.clone()).await;
            },
            None => {
                error!("Received message from unknown peer");
            }
        }
    }
}
