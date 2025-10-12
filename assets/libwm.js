class WindowManager {
    constructor() {
        this.windows = {};      // all registered windows
        this.zCounter = 1000;   // z-index counter
        this.activeWindow = null;
        this.dragInfo = null;
        this.resizeInfo = null;

        // Bind handlers
        this.handleDrag = this.handleDrag.bind(this);
        this.stopDrag = this.stopDrag.bind(this);
        this.handleResize = this.handleResize.bind(this);
        this.stopResize = this.stopResize.bind(this);

        // Initialize
        this.initStaticWindows();
        this.initTaskbar();
        this.initStartMenu();
    }

    initStaticWindows() {
        const wm = this;
        document.querySelectorAll(".mdi-window").forEach(win => {
            wm.registerWindow(win);
            win.addEventListener("mousedown", e => {
                if (!e.target.classList.contains("resize-handle")) {
                    wm.bringToFront(win.id);
                }
            });
        });
    }

    registerWindow(win) {
        if (!win.id) return;
        this.windows[win.id] = win;
        win.dataset.maximized = "false";
    }

    bringToFront(id) {
        const win = this.windows[id];
        if (!win) return;

        this.zCounter++;
        win.style.zIndex = this.zCounter;

        // Mark active/inactive windows
        Object.values(this.windows).forEach(w => w.classList.remove("active"));
        win.classList.add("active");
        this.activeWindow = win;

        // Update taskbar highlights
        document.querySelectorAll('.taskbar-item').forEach(item => item.classList.remove('active'));
        const taskItem = document.querySelector(`.taskbar-item[data-window="${id}"]`);
        if (taskItem) taskItem.classList.add('active');
    }

    initTaskbar() {
        const wm = this;
        document.querySelectorAll(".taskbar-item").forEach(item => {
            item.addEventListener("click", () => {
                const id = item.dataset.window;
                const win = wm.windows[id];
                if (!win) return;

                if (win.style.display === "none") {
                    win.style.display = "block";
                }
                wm.bringToFront(id);
            });
        });
    }

    initStartMenu() {
        const wm = this;
        const startButton = document.querySelector('.start-button');
        const startMenu = document.querySelector('.start-menu');
        if (!startButton || !startMenu) return;

        startButton.addEventListener('click', e => {
            e.stopPropagation();
            startMenu.style.display = startMenu.style.display === 'block' ? 'none' : 'block';
        });

        document.addEventListener('click', () => {
            startMenu.style.display = 'none';
        });

        startMenu.querySelectorAll('.start-menu-item').forEach(item => {
            item.addEventListener('click', e => {
                const name = e.target.innerText;
                startMenu.style.display = 'none';
                const winId = `window-${name.toLowerCase().replace(/\s+/g, '-')}`;
                const win = wm.windows[winId];
                if (win) {
                    win.style.display = 'block';
                    wm.bringToFront(winId);
                }
            });
        });
    }

    // --- Dragging ---
    startDrag(event, id) {
		event.preventDefault();
        const win = this.windows[id];
        if (!win) return;

        this.bringToFront(id);

        this.dragInfo = {
            win,
            offsetX: event.clientX - win.offsetLeft,
            offsetY: event.clientY - win.offsetTop
        };

        document.addEventListener("mousemove", this.handleDrag);
        document.addEventListener("mouseup", this.stopDrag);
    }

    handleDrag(event) {
        if (!this.dragInfo) return;
        const { win, offsetX, offsetY } = this.dragInfo;
        win.style.left = `${event.clientX - offsetX}px`;
        win.style.top = `${event.clientY - offsetY}px`;
    }

    stopDrag() {
        document.removeEventListener("mousemove", this.handleDrag);
        document.removeEventListener("mouseup", this.stopDrag);
        this.dragInfo = null;
    }

    // --- Resize ---
    startResize(event, id, direction) {
		event.preventDefault();
        const win = this.windows[id];
        if (!win) return;

        this.bringToFront(id);

        this.resizeInfo = {
            win,
            direction,
            startX: event.clientX,
            startY: event.clientY,
            startWidth: win.offsetWidth,
            startHeight: win.offsetHeight,
            startLeft: win.offsetLeft,
            startTop: win.offsetTop
        };

        document.addEventListener("mousemove", this.handleResize);
        document.addEventListener("mouseup", this.stopResize);
    }

    handleResize(event) {
        if (!this.resizeInfo) return;
        const { win, direction, startX, startY, startWidth, startHeight, startLeft, startTop } = this.resizeInfo;
        let newWidth = startWidth;
        let newHeight = startHeight;
        let newLeft = startLeft;
        let newTop = startTop;

        if (direction.includes("e")) newWidth = startWidth + (event.clientX - startX);
        if (direction.includes("s")) newHeight = startHeight + (event.clientY - startY);
        if (direction.includes("w")) {
            newWidth = startWidth - (event.clientX - startX);
            newLeft = startLeft + (event.clientX - startX);
        }
        if (direction.includes("n")) {
            newHeight = startHeight - (event.clientY - startY);
            newTop = startTop + (event.clientY - startY);
        }

        win.style.width = `${Math.max(newWidth, 200)}px`;
        win.style.height = `${Math.max(newHeight, 150)}px`;
        win.style.left = `${newLeft}px`;
        win.style.top = `${newTop}px`;
    }

    stopResize() {
        document.removeEventListener("mousemove", this.handleResize);
        document.removeEventListener("mouseup", this.stopResize);
        this.resizeInfo = null;
    }

    maximizeWindow(id) {
        const win = this.windows[id];
        if (!win) return;

        if (win.dataset.maximized === "true") {
            // Restore
            win.style.left = win.dataset.left;
            win.style.top = win.dataset.top;
            win.style.width = win.dataset.width;
            win.style.height = win.dataset.height;
            win.dataset.maximized = "false";
        } else {
            // Save original
            win.dataset.left = win.style.left;
            win.dataset.top = win.style.top;
            win.dataset.width = win.style.width;
            win.dataset.height = win.style.height;

            win.style.left = "0px";
            win.style.top = "0px";
            win.style.width = "100vw";
            win.style.height = "100vh";
            win.dataset.maximized = "true";
        }
    }

    closeWindow(id) {
        const win = this.windows[id];
        if (!win) return;

        // Abort any HTMX SSE/polling
        win.querySelectorAll("[hx-sse], [hx-get][hx-trigger]").forEach(el => {
            if (el._htmx) el._htmx.abort?.();
        });

        win.style.display = "none";

        const taskItem = document.querySelector(`.taskbar-item[data-window="${id}"]`);
        if (taskItem) taskItem.classList.remove("active");
    }

    // --- Create windows from HTMX responses ---
    createWindowFromHTMX(id, contentHTML) {
        if (this.windows[id]) {
            const win = this.windows[id];
            win.innerHTML = contentHTML;
            win.style.display = "block";
            this.bringToFront(id);
        } else {
            const win = document.createElement("div");
            win.id = id;
            win.className = "mdi-window";
            win.innerHTML = contentHTML;
            win.style.position = "absolute";
            win.style.left = "100px";
            win.style.top = "100px";
            win.style.width = "400px";
            win.style.height = "300px";
            document.body.appendChild(win);
            this.registerWindow(win);
            this.bringToFront(id);
        }
    }
}

// Usage
document.addEventListener("DOMContentLoaded", () => {
    window.wm = new WindowManager();
	window.startDrag = (e, id) => wm.startDrag(e, id);
    window.startResize = (e, id, dir) => wm.startResize(e, id, dir);
    window.maximizeWindow = (id) => wm.maximizeWindow(id);
    window.closeWindow = (id) => wm.closeWindow(id);
});

