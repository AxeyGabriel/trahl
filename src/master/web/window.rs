use maud::{html, Markup};

pub fn create_window(
    id: &str,
    title: &str,
    style: &str,
    resizeable: bool,
    content: Markup
) -> Markup {
    html! {
        div.mdi-window id=(id) style=(style) data-title=(title) data-resizeable=(resizeable) {
            (window_title(id, title))
            @if resizeable {
                (resize_handles(id))
            }
            div.window-content {
                (content)
            }
        }
    }
}

fn window_title(window_id: &str, title: &str) -> Markup {
    html! {
        div.title-bar onmousedown=(format!("startDrag(event, '{}')", window_id)) {
            span.title-text { (title) }
            div.title-buttons {
                button.title-button onclick=(format!("maximizeWindow('{}')", window_id)) { "□" }
                button.title-button onclick=(format!("closeWindow('{}')", window_id)) { "✕" }
            }
        }
    }
}

fn resize_handles(window_id: &str) -> Markup {
    html! {
        div.resize-handle.resize-n onmousedown=(format!("startResize(event, '{}', 'n')", window_id)) {}
        div.resize-handle.resize-s onmousedown=(format!("startResize(event, '{}', 's')", window_id)) {}
        div.resize-handle.resize-e onmousedown=(format!("startResize(event, '{}', 'e')", window_id)) {}
        div.resize-handle.resize-w onmousedown=(format!("startResize(event, '{}', 'w')", window_id)) {}
        div.resize-handle.resize-ne onmousedown=(format!("startResize(event, '{}', 'ne')", window_id)) {}
        div.resize-handle.resize-nw onmousedown=(format!("startResize(event, '{}', 'nw')", window_id)) {}
        div.resize-handle.resize-se onmousedown=(format!("startResize(event, '{}', 'se')", window_id)) {}
        div.resize-handle.resize-sw onmousedown=(format!("startResize(event, '{}', 'sw')", window_id)) {}
    }
}

pub fn create_content(content: Markup) -> Markup {
    html! {
        div.window-content {
            (content)
        }
    }
}
