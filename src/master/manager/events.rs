#[derive(Clone)]
pub enum ManagerEvent {
    PeerList { },
    JobQueue(Vec<JobQueueEntry>),
}

#[derive(Clone)]
pub struct JobQueueEntry {
    pub file: String,
    pub library: String,
    pub worker: String,
    pub status: String,
    pub milestone: String,
    pub progress: String,
    pub eta: String,
}
