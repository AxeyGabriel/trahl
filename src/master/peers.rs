use std::collections::HashMap;
use std::time::Instant;

#[derive(Debug)]
pub struct PeerInfo {
    pub identifier: String,
    pub last_seen: Instant,
    pub simultaneous_jobs: u8,
}

pub type PeerId = Vec<u8>;

#[derive(Debug)]
pub struct PeerRegistry {
    pub peers: HashMap<PeerId, PeerInfo>
}

impl Default for PeerRegistry {
    fn default() -> Self {
        PeerRegistry { peers: HashMap::new() }
    }
}
