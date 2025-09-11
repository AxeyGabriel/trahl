use std::collections::{HashMap, VecDeque};
use std::time::Instant;

use crate::rpc::Message;

#[derive(Debug)]
pub struct PeerInfo {
    pub identifier: String,
    pub last_seen: Instant,
    pub simultaneous_jobs: u8,
    pub handshake_state: Handshake,
    pub tx_ringbuffer:  VecDeque<Message>,
}

pub type PeerId = Vec<u8>;

#[derive(Debug)]
pub struct PeerRegistry {
    pub peers: HashMap<Vec<u8>, PeerInfo>
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

    pub fn add(&mut self, id: PeerId, peer: PeerInfo) {
        self.peers.insert(id, peer);
    }

    pub fn remove(&mut self, id: &[u8]) {
        let _ = self.peers.remove(id);
    }

    pub fn get(&self, id: &[u8]) -> Option<&PeerInfo> {
        self.peers
            .get(id)
    }
    
    pub fn get_mut(&mut self, id: &[u8]) -> Option<&mut PeerInfo> {
        self.peers
            .get_mut(id)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&[u8], &PeerInfo)> {
        self.peers
            .iter()
            .map(|(k,v)| (k.as_slice(), v))
    }

    pub fn poll_all(&mut self) -> Vec<(PeerId, Message)> {
        self.peers
            .iter_mut()
            .filter_map(|(id, peer)| {
                peer.poll()
                    .map(|msg| (id.clone(), msg))
            })
            .collect()
    }
}

#[derive(Debug)]
enum Handshake {
    Disconnected,
    HelloSent,
    HelloAckSent,
    Ready,
}

impl Default for PeerInfo {
    fn default() -> Self {
        PeerInfo {
            identifier: "dummy".to_string(),
            simultaneous_jobs: 1,
            last_seen: Instant::now(),
            handshake_state: Handshake::Disconnected,
            tx_ringbuffer: VecDeque::default(),
        }
    }
}

impl PeerInfo {
    pub async fn on_message(&mut self, msg: &Message) {
        match msg {
            Message::Hello(p) => {
                self.handshake_state = Handshake::HelloSent;
                self.tx_ringbuffer
                    .push_back(Message::HelloAck);
            }
            _ => {},
        }
    }

    pub fn poll(&mut self) -> Option<Message> {
        self.tx_ringbuffer
            .pop_front()
    }
}
