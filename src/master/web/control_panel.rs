use super::window;
use maud::{html, Markup};

pub fn window() -> Markup {
    let content = window::create_content(html! {
        div.control-section {
            div.panel {
                h3 { "QUICK STATS" }
                div.quick-stats {
                    div { "CPU Usage: 78%" }
                    div { "Memory: 4.2/16 GB" }
                    div { "Disk I/O: 145 MB/s" }
                    div { "Network: 23 MB/s" }
                    div { "Counter: "
                        div hx-ext="sse" sse-connect="/sse/manager_events" sse-swap="PeerList" #counter {}
                    }
                }
            }
        }
    });

    let statusbar = html! {
        div.status-bar {
            //When adding a status bar, adjust window-content height to: calc(100% - 44px)
            div class="status-bar-item flex-grow" { "Item 1" }
            div.status-bar-separator { }
            div.status-bar-item { "Item 2" }
        }
    };

    let window_content = html! {
        (content)
        (statusbar)
    };

    window::create_window(
        "window-control",
        "Control Panel",
        "left: 640px; top: 420px; width: 360px; height: 380px;",
        false,
        window_content
    )
}
