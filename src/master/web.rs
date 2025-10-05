use tokio::time::{sleep, Duration};
use std::sync::Arc;

use super::MasterCtx;

pub async fn web_service(ctx: Arc<MasterCtx>) {
    let ch_term = &ctx.ch_terminate.1;

    loop {
        if let Ok(_) = ch_term.has_changed() {
            if *ch_term.borrow() {
                break;
            }
        }
//        info!("{:#?}", ctx.config.read().unwrap().jobs);
        sleep(Duration::from_secs(1)).await;
    }
}
