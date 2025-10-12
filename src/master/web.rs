mod sse;
mod windows;
mod index;

use axum::{
    http,
    Router,
    routing::get,
    response::IntoResponse,
};
use tower_http::trace::{
    TraceLayer,
    DefaultMakeSpan,
    DefaultOnResponse,
};
use maud::{html, Markup};
use reqwest::header;
use std::sync::Arc;
use tokio::net::TcpListener;
use tracing::{info, Level};

use super::MasterCtx;

include!(concat!(env!("OUT_DIR"), "/assets.rs"));

//const WEB_UI_STYLE: &'static str = ASSETS_STYLE_RED_CSS;
//const WEB_UI_STYLE: &'static str = ASSETS_STYLE_GRAY_CSS;
const WEB_UI_STYLE: &'static str = ASSETS_STYLE_W98_CSS;
//const WEB_UI_STYLE: &'static str = ASSETS_STYLE_NEON_CSS;

pub async fn web_service(ctx: Arc<MasterCtx>) {
    let master_config = {
        let cfg = &ctx.config
        .read()
        .unwrap();
        cfg.master.clone()
    };

    let router = Router::new()
            .route("/sse/clock", get(sse::clock))
            .route("/", get(index::index()))
            .route("/windows/window-queue", get(queue_window()))
            .route("/windows/window-control", get(control_panel_window()))
            .route("/windows/window-activity", get(activity_window()))
            .route("/windows/window-statistics", get(statistics_window()))
            .route("/favicon.ico", get(|| async {
                axum::response::Redirect::permanent("/static/favicon.ico")
            }))
            .route("/static/htmx.min.js", get(|| async { serve_cached_asset(ASSETS_HTMX_MIN_2_0_7_JS, "application/javascript") } ))
            .route("/static/htmx-ext-sse.min.js", get(|| async { serve_cached_asset(ASSETS_HTMX_EXT_SSE_MIN_2_2_2_JS, "application/javascript") } ))
            .route("/static/style.css", get(|| async { serve_asset(WEB_UI_STYLE, "text/css") } ))
            .route("/static/libwm.js", get(|| async { serve_asset(ASSETS_LIBWM_JS, "application/javascript") } ))
            .route("/static/favicon.ico", get(|| async { serve_binary_asset(ASSETS_FAVICON_ICO, "image/x-icon") } ))
            .layer(TraceLayer::new_for_http()
                .make_span_with(DefaultMakeSpan::new().level(Level::INFO))
                .on_response(DefaultOnResponse::new().level(Level::INFO)));

    let listener = TcpListener::bind(master_config.web_bind_addr).await;
    match listener {
        Ok(listener) => {
            info!("WEB interface listening at {}", master_config.web_bind_addr);
            if let Err(e) = axum::serve(listener, router).await {
                panic!("Error serving web: {}", e);
            }
        },
        Err(e) => {
            panic!("Cannot bind at {}: {}", master_config.web_bind_addr, e);
        }
    }
}

fn serve_binary_asset(content: &'static [u8], content_type: &'static str) -> impl IntoResponse {
    (
        [(header::CACHE_CONTROL, http::HeaderValue::from_static("public, max-age=2592000, immutable"))],
        [(http::header::CONTENT_TYPE, content_type)],
        content,
    )
}

fn serve_asset(content: &'static str, content_type: &'static str) -> impl IntoResponse {
    (
        [(header::CACHE_CONTROL, http::HeaderValue::from_static("no-cache"))],
        [(http::header::CONTENT_TYPE, content_type)],
        content,
    )
}

fn serve_cached_asset(content: &'static str, content_type: &'static str) -> impl IntoResponse {
    (
        [(header::CACHE_CONTROL, http::HeaderValue::from_static("public, max-age=2592000, immutable"))],
        [(http::header::CONTENT_TYPE, content_type)],
        content,
    )
}

fn statistics_window() -> Markup {
    let content = html! {
        div.window-content
            style="height: calc(100% - 22px);" {
            (stats_content())
        }
    };

    windows::create_window(
        "window-statistics",
        "Statistics",
        "left: 20px; top: 20px; width: 480px; height: 280px; z-index: 1004;",
        content
    )
}

fn stats_content() -> Markup {
    html! {
        div.stats-grid {
            div.stat-card {
                div.stat-label {
                    "TOTAL FILES"
                    span.stat-icon style="background: #00ff00;" { "✓" }
                }
                div.stat-value { "12,847" }
                div.stat-detail { "+324 today" }
            }
            div.stat-card {
                div.stat-label {
                    "QUEUE"
                    span.stat-icon style="background: #0000ff; color: white;" { "⏱" }
                }
                div.stat-value { "45" }
                div.stat-detail { "12 processing" }
            }
            div.stat-card {
                div.stat-label {
                    "FAILED"
                    span.stat-icon style="background: #ff0000; color: white;" { "⚠ " }
                }
                div.stat-value { "7" }
                div.stat-detail { "2 this week" }
            }
            div.stat-card {
                div.stat-label {
                    "WORKERS ACTIVE"
                    span.stat-icon style="background: #ffff00;" { "⚡" }
                }
                div.stat-value { "8/12" }
                div.stat-detail { "66% utilization" }
            }
        }
    }
}

fn queue_window() -> Markup {
    let content = html! {
        div.window-content style="height: calc(100% - 22px);" {
            table.win98-table {
                thead {
                    tr {
                        th { "FILE" }
                        th { "STATUS" }
                        th { "PROGRESS" }
                        th { "WORKER" }
                        th { "LIBRARY" }
                        th { "ETA" }
                    }
                }
                tbody # queue-tbody {
                    (queue_rows())
                }
            }
        }
    };
    
    windows::create_window(
        "window-queue",
        "Transcode Queue",
        "left: 520px; top: 20px; width: 600px; height: 380px; z-index: 1003;",
        content
    )
}

fn queue_rows() -> Markup {
    html! {
        tr {
            td { "movie_2023_4k.mkv" }
            td { span.status-badge.status-processing { "PROCESSING" } }
            td {
                div.progress-bar {
                    div.progress-fill style="width: 67%;" {}
                }
                span { "67%" }
            }
            td { "Worker-03" }
            td { "Movies" }
            td { "12m 34s" }
        }
        tr {
            td { "series_s01e05.mp4" }
            td { span.status-badge.status-queued { "QUEUED" } }
            td { "-" }
            td { "-" }
            td { "TV Shows" }
            td { "Pending" }
        }
        tr {
            td { "documentary_hd.avi" }
            td { span.status-badge.status-processing { "PROCESSING" } }
            td {
                div.progress-bar {
                    div.progress-fill style="width: 23%;" {}
                }
                span { "23%" }
            }
            td { "Worker-07" }
            td { "Documentaries" }
            td { "45m 12s" }
        }
    }
}

fn activity_window() -> Markup {
    let content = html! {
        div.window-content # activity-feed
            style="height: calc(100% - 22px);" {
            (activity_items())
        }
    };
    
    windows::create_window(
        "window-activity",
        "Activity",
        "left: 120px; top: 320px; width: 500px; height: 340px; z-index: 1002;",
        content
    )
}

fn activity_items() -> Markup {
    html! {
        div.activity-item {
            div.activity-icon style="background: #00ff00;" { "✓" }
            div.activity-content {
                div style="display: flex; justify-content: space-between;" {
                    div style="flex: 1;" {
                        div.activity-title { "Successfully transcoded movie_2023_4k.mkv" }
                        div.activity-detail { "H.264 → H.265 | Size reduced by 34%" }
                    }
                    div.activity-meta {
                        div { span.status-badge.status-success { "SUCCESS" } }
                        div.activity-time { "14:32" }
                    }
                }
            }
        }
        div.activity-item {
            div.activity-icon style="background: #ff0000; color: white;" { "✗" }
            div.activity-content {
                div style="display: flex; justify-content: space-between;" {
                    div style="flex: 1;" {
                        div.activity-title { "Failed to transcode animation_film.mkv" }
                        div.activity-detail { "Error: Codec not supported" }
                    }
                    div.activity-meta {
                        div { span.status-badge.status-error { "ERROR" } }
                        div.activity-time { "14:29" }
                    }
                }
            }
        }
    }
}

fn control_panel_window() -> Markup {
    let content = html! {
        div.window-content style="height: calc(100% - 22px);" {
            div.control-section {
                div.win98-panel {
                    h3 { "SYSTEM CONTROLS" }
                    div.button-group {
                        button.win98-button {
                            "Start All Workers"
                        }
                        button.win98-button {
                            "Pause Queue"
                        }
                        button.win98-button {
                            "Clear Failed Jobs"
                        }
                    }
                }
            }

            div.control-section {
                div.win98-panel {
                    h3 { "QUICK STATS" }
                    div.quick-stats {
                        div { "CPU Usage: 78%" }
                        div { "Memory: 4.2/16 GB" }
                        div { "Disk I/O: 145 MB/s" }
                        div { "Network: 23 MB/s" }
                    }
                }
            }
        }
    };

    windows::create_window(
        "window-control",
        "Control Panel",
        "left: 640px; top: 420px; width: 360px; height: 300px; z-index: 1001;",
        content
    )
}
