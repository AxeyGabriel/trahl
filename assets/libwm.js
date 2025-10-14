// --- Individual window object ---
class Window {
	constructor(dom) {
		this.dom = dom;
		this.id = dom.id;
		this.name = dom.dataset.title || dom.id.replace('window-', '');

		const style = window.getComputedStyle(dom);
		this.x = parseInt(style.left) || Math.floor((window.innerWidth - this.width) / 2);
		this.y = parseInt(style.top) || Math.floor((window.innerHeight - this.height) / 2);
		this.width = parseInt(style.width) || 400;
		this.height = parseInt(style.height) || 300;
		this.maximized = dom.dataset.maximized === "true";
		this.zIndex = parseInt(style.zIndex) || 1000;

		this.restoreFromStorage();
		this.updateDOM();
	}

	updateDOM() {
		if (!this.dom) return;

		this.dom.dataset.maximized = this.maximized ? "true" : "false";
		
		if (this.maximized) {
			Object.assign(this.dom.style, {
				left: "0px",
				top: "0px",
				width: "100vw",
				height: "100vh",
				zIndex: this.zIndex
			});
		} else {
			Object.assign(this.dom.style, {
				left: `${this.x}px`,
				top: `${this.y}px`,
				width: `${this.width}px`,
				height: `${this.height}px`,
				zIndex: this.zIndex
			});
		}	 
	}

	saveToStorage() {
		const data = {
			x: this.x,
			y: this.y,
			width: this.width,
			height: this.height,
			maximized: this.maximized,
			zIndex: this.zIndex
		};
		const all = JSON.parse(localStorage.getItem("windowState") || "{}");
		all[this.id] = data;
		localStorage.setItem("windowState", JSON.stringify(all));
	}

	restoreFromStorage() {
		try {
			const all = JSON.parse(localStorage.getItem("windowState") || "{}");
			const s = all[this.id];
			if (s) {
				this.x = s.x;
				this.y = s.y;
				this.width = s.width;
				this.height = s.height;
				this.maximized = s.maximized;
				this.zIndex = s.zIndex;
			}
		} catch {
			console.warn("Could not restore window state");
		}
	}

	removeFromStorage() {
		const all = JSON.parse(localStorage.getItem("windowState") || "{}");
		delete all[this.id];
		localStorage.setItem("windowState", JSON.stringify(all));
	}
}

// --- Window Manager ---
class WindowManager {
	constructor() {
		this.windows = new Map();
		this.zCounter = 1000;
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
		this.restoreOpenWindows();
	}

	// --- Persist open windows ---
	saveOpenWindows() {
		localStorage.setItem("openWindows", JSON.stringify([...this.windows.keys()]));
	}

	async restoreOpenWindows() {
		const saved = JSON.parse(localStorage.getItem("openWindows") || "[]");
		for (const id of saved) {
			if (!this.windows.has(id)) {
				var winObj = await this.fetchWindow(id);
				if (winObj && winObj.zIndex >= this.zCounter) {
					this.zCounter = winObj.zIndex;
					this.windows.forEach(w => w.dom.classList.remove("active"));
					winObj.dom.classList.add("active");
					this.activeWindow = winObj;
					this.updateTaskbar();
				}
			}
		}
	}

	// --- Initialization of static windows (HTML preloaded) ---
	initStaticWindows() {
		document.querySelectorAll(".mdi-window:not(.modal)").forEach(dom => {
			this.registerWindow(new Window(dom));
		});
	}

	// --- Register a window ---
	registerWindow(winObj) {
		if (!winObj.id) return;
		this.windows.set(winObj.id, winObj);
		const dom = winObj.dom;

		// Bring to front when clicking anywhere except resize handles
		dom.addEventListener("mousedown", e => {
			if (!e.target.classList.contains("resize-handle")) {
				this.bringToFront(winObj);
			}
			e.preventDefault(); // prevent random text selection
		});

		// Wire close/maximize buttons
		const closeBtn = dom.querySelector(".window-btn.close");
		if (closeBtn) closeBtn.addEventListener("click", () => this.closeWindow(winObj));

		const maxBtn = dom.querySelector(".window-btn.maximize");
		if (maxBtn) maxBtn.addEventListener("click", () => this.maximizeWindow(winObj));

		winObj.updateDOM();
		this.saveOpenWindows();
		this.updateTaskbar();
	}

	// --- Bring window to front ---
	bringToFront(winObj) {
		if (!winObj || !winObj.dom) return;
		var doIt = false;
		this.windows.forEach(wo => {
			if (wo.id != winObj.id) {
				if (wo.zIndex >= winObj.zIndex) {
					doIt = true;
				}
			}
		});
		if (!doIt) return;
		this.zCounter++;
		winObj.zIndex = this.zCounter;
		winObj.updateDOM();
		winObj.saveToStorage();

		// Mark active
		this.windows.forEach(w => w.dom.classList.remove("active"));
		winObj.dom.classList.add("active");
		this.activeWindow = winObj;

		this.updateTaskbar();
	}

	// --- Taskbar behavior ---
	initTaskbar() {
		this.updateTaskbar();
	}

	updateTaskbar() {
		const bar = document.querySelector('.taskbar-items');
		if (!bar) return;
		bar.innerHTML = '';
		this.windows.forEach(winObj => {
			const item = document.createElement('div');
			item.className = 'taskbar-item';
			item.dataset.window = winObj.id;
			item.textContent = winObj.name;
			if (winObj.dom.classList.contains('active')) item.classList.add('active');
			item.addEventListener('click', () => this.bringToFront(winObj));
			bar.appendChild(item);
		});
	}

	// --- Start Menu behavior ---
	initStartMenu() {
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
			item.addEventListener('click', async e => {
				e.stopPropagation();
				startMenu.style.display = 'none';
				const id = item.dataset.window;
				if (!this.windows.has(id)) {
					var winObj = await this.fetchWindow(id);
					this.bringToFront(winObj);
				} else {
					this.bringToFront(this.windows.get(id));
				}
			});
		});
	}

	// --- Dragging ---
	startDrag(event, winObj) {
		event.preventDefault();
		this.bringToFront(winObj);
		this.dragInfo = {
			winObj,
			offsetX: event.clientX - winObj.dom.offsetLeft,
			offsetY: event.clientY - winObj.dom.offsetTop
		};
		document.body.style.userSelect = "none";
		document.addEventListener("mousemove", this.handleDrag);
		document.addEventListener("mouseup", this.stopDrag);
	}

	handleDrag(event) {
		if (!this.dragInfo) return;
		const { winObj, offsetX, offsetY } = this.dragInfo;
		winObj.x = event.clientX - offsetX;
		winObj.y = event.clientY - offsetY;
		winObj.updateDOM();
	}

	stopDrag() {
		document.body.style.userSelect = "";
		document.removeEventListener("mousemove", this.handleDrag);
		document.removeEventListener("mouseup", this.stopDrag);
		if (this.dragInfo) this.dragInfo.winObj.saveToStorage();
		this.dragInfo = null;
	}

	// --- Resizing ---
	startResize(event, winObj, dir) {
		event.preventDefault();
		this.bringToFront(winObj);
		const rect = winObj.dom.getBoundingClientRect();
		this.resizeInfo = {
			winObj, dir,
			startX: event.clientX,
			startY: event.clientY,
			startWidth: rect.width,
			startHeight: rect.height,
			startLeft: rect.left,
			startTop: rect.top
		};
		document.body.style.userSelect = "none";
		document.addEventListener("mousemove", this.handleResize);
		document.addEventListener("mouseup", this.stopResize);
	}

	handleResize(event) {
		if (!this.resizeInfo) return;
		const { winObj, dir, startX, startY, startWidth, startHeight, startLeft, startTop } = this.resizeInfo;

		let dx = event.clientX - startX;
		let dy = event.clientY - startY;

		if (dir.includes('e')) winObj.width = startWidth + dx;
		if (dir.includes('s')) winObj.height = startHeight + dy;
		if (dir.includes('w')) { winObj.width = startWidth - dx; winObj.x = startLeft + dx; }
		if (dir.includes('n')) { winObj.height = startHeight - dy; winObj.y = startTop + dy; }

		winObj.width = Math.max(winObj.width, 200);
		winObj.height = Math.max(winObj.height, 150);
		winObj.updateDOM();
	}

	stopResize() {
		document.body.style.userSelect = "";
		document.removeEventListener("mousemove", this.handleResize);
		document.removeEventListener("mouseup", this.stopResize);
		if (this.resizeInfo) this.resizeInfo.winObj.saveToStorage();
		this.resizeInfo = null;
	}

	// --- Maximize toggle ---
	maximizeWindow(winObj) {
		const dom = winObj.dom;
		if (dom.dataset.maximized === "true") {
			winObj.maximized = false;
		} else {
			dom.dataset.left = dom.style.left;
			dom.dataset.top = dom.style.top;
			dom.dataset.width = dom.style.width;
			dom.dataset.height = dom.style.height;
			winObj.maximized = true;
		}
		winObj.updateDOM();
		winObj.saveToStorage();
	}

	// --- Close window ---
	closeWindow(winObj) {
		const dom = winObj.dom;

		// Abort htmx SSE & polling
		dom.querySelectorAll("[hx-ext='sse'], [hx-trigger*='every']").forEach(el => {
			if (el._htmx_sse_source?.close) el._htmx_sse_source.close();
			if (el._htmxInterval) clearInterval(el._htmxInterval);
		});

		dom.remove();
		this.windows.delete(winObj.id);
		this.saveOpenWindows();
		this.updateTaskbar();
	}

	// --- Fetch new window ---
	async fetchWindow(id) {
		try {
			const resp = await fetch(`/windows/${id}`, { headers: { 'HX-Request': 'true' } });
			if (!resp.ok) throw new Error(`HTTP ${resp.status}`);
			const html = await resp.text();
			const container = document.createElement('div');
			container.innerHTML = html.trim();
			const dom = container.firstElementChild;
			if (!dom?.classList.contains('mdi-window')) {
				console.warn(`Invalid window HTML from /windows/${id}`);
				return;
			}
			document.body.appendChild(dom);

			const winObj = new Window(dom);
			this.registerWindow(winObj);
			htmx.process(dom);

			return winObj;
		} catch (err) {
			console.error("Failed to load window:", err);
		}
	}

	openModal(modalId) {
			const modal = document.getElementById(modalId);
			const overlay = document.getElementById('modal-overlay');

			const allWindows = document.querySelectorAll('.mdi-window:not(.modal)');
			allWindows.forEach(window => {
				window.classList.add('disabled');
			});

			overlay.classList.add('active');
			modal.style.display = 'block';
	}

	closeModal(modalId) {
		const modal = document.getElementById(modalId);
		const overlay = document.getElementById('modal-overlay');

		const allWindows = document.querySelectorAll('.mdi-window:not(.modal)');
		allWindows.forEach(window => {
			window.classList.remove('disabled');
		});

		overlay.classList.remove('active');
		modal.style.display = 'none';
	}
}

// --- Global init ---
document.addEventListener("DOMContentLoaded", () => {
	window.wm = new WindowManager();

	window.startDrag = (e, id) => wm.startDrag(e, wm.windows.get(id));
	window.startResize = (e, id, dir) => wm.startResize(e, wm.windows.get(id), dir);
	window.maximizeWindow = id => wm.maximizeWindow(wm.windows.get(id));
	window.closeWindow = id => wm.closeWindow(wm.windows.get(id));
});

document.addEventListener('mousedown', e => {
	if (e.target.matches('.button')) {
		e.target.classList.add('pressed');
	}
});

document.addEventListener('mouseup', e => {
	document.querySelectorAll('.pressed').forEach(btn => btn.classList.remove('pressed'));
});

// --- Connection failure detection ---
let consecutiveClockErrors = 0;
const maxErrors = 2;

document.body.addEventListener('htmx:sseError', function(event) {
    const target = event.target;
    if (target && target.id === 'clock') {
        consecutiveClockErrors++;

        if (consecutiveClockErrors >= maxErrors) {
            wm.openModal("modal-lostconn");
        }
    }
});

document.body.addEventListener('htmx:sseMessage', function(event) {
    const target = event.target;
    if (target && target.id === 'clock') {
        consecutiveClockErrors = 0;
    }
});
