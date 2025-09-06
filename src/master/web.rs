use tokio::time::{sleep, Duration};
use tokio::sync::watch;
use tracing::info;

use crate::CONFIG;

pub async fn web_service(ch_term: watch::Receiver<bool>) {
    loop {
        if let Ok(_) = ch_term.has_changed() {
            if *ch_term.borrow() {
                break;
            }
        }
        info!("web");
        sleep(Duration::from_secs(1)).await;
    }
}
