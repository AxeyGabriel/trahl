use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::info;
use uuid::Uuid;

use crate::master::peers::{PeerId, RxManagerMsg};
use crate::rpc::Message;
use super::MasterCtx;
use super::socket_server::SocketEvent;
use super::peers::TxManagerMsg;

struct PeerInfo {
    tx: mpsc::Sender<RxManagerMsg>,
    simultaneous_jobs: u8,
    identifier: String,
    state: PeerState,
}

enum PeerState {

}

pub type JobResult = Result<(), String>;

struct JobSpec {
    id: Uuid,
    script: String,
    vars: Option<HashMap<String, String>>,
    result_tx: oneshot::Sender<JobResult>,
    log_tx: mpsc::Sender<String>,
    //tracing

}

pub struct TaskManager {
    rx_from_peer: mpsc::Receiver<TxManagerMsg>,
    rx_socket_events: mpsc::Receiver<SocketEvent>,
    peer_registry: HashMap<PeerId, PeerInfo>,
// pending_jobs: VecDeque<JobSpec>
// running_jobs: HashMap<Uuid, PeerId>
}

impl TaskManager {
    pub fn new(
        rx_from_peer: mpsc::Receiver<TxManagerMsg>,
        rx_socket_events: mpsc::Receiver<SocketEvent>,
    ) -> Self {
        Self {
            rx_from_peer,
            rx_socket_events,
            peer_registry: HashMap::new(),
        }
    }

    pub async fn run(mut self, ctx: Arc<MasterCtx>) {
        let mut ch_term = ctx.ch_terminate.1.clone();
        let mut ch_reload = ctx.ch_reload.1.clone();

        loop {
            tokio::select!(
                msg = self.rx_from_peer.recv() => {
                    if let Some(msg) = msg {
                        info!("task_driver: {:#?}", msg);
                    }
                },
                Some(event) = self.rx_socket_events.recv() => {
                    match event {
                        SocketEvent::PeerConnected(peer_id, tx) => {
                            let peer_info = PeerInfo {
                                tx
                            };
                            self.peer_registry
                                .insert(peer_id, peer_info);
                        },
                        SocketEvent::PeerDisconnected(peer_id) => {
                            self.peer_registry
                                .remove(&peer_id);
                        }
                    }
                },
                _ = ch_reload.changed() => {
                    if *ch_reload.borrow() {
                        //todo! send configupdate
                    }
                },
                _ = ch_term.changed() => {
                    if *ch_term.borrow() {
                        break;
                    }
                }
            );
        }
    }
}

