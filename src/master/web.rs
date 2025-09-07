use tokio::time::{sleep, Duration};
use tokio::sync::watch;
use std::sync::Arc;
use tracing::info;

use super::MasterCtx;

pub async fn web_service(ctx: Arc<MasterCtx>) {
    let ch_term = &ctx.ch_terminate.1;

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
