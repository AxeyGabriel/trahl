let zCounter = 1000;
let activeWindow = null;

function bringToFront(id) {
	const win = document.getElementById(id);
	if (!win) return;

	zCounter++;
	win.style.zIndex = zCounter;

	// Mark active/inactive
	document.querySelectorAll('.mdi-window').forEach(w => w.classList.remove('active'));
	win.classList.add('active');
	activeWindow = win;

	// Update taskbar highlight
	document.querySelectorAll('.taskbar-item').forEach(item => item.classList.remove('active'));
	const taskItem = document.querySelector(`.taskbar-item[data-window="${id}"]`);
	if (taskItem) taskItem.classList.add('active');
}

document.addEventListener("DOMContentLoaded", () => {
	document.querySelectorAll(".mdi-window").forEach(win => {
		win.addEventListener("mousedown", e => {
			if (!e.target.classList.contains("resize-handle")) {
				bringToFront(win.id);
			}
		});
	});

	document.querySelectorAll(".taskbar-item").forEach(item => {
		item.addEventListener("click", () => {
			const id = item.dataset.window;
			const win = document.getElementById(id);
			if (!win) return;

			const isVisible = win.style.display !== "none";

			if (isVisible) {
				bringToFront(id);
			} else {
				win.classList.remove("active");
				win.style.display = "block";
				bringToFront(id);
			}
		});
	});
});

// --- Dragging ---
let dragInfo = null;

function startDrag(event, id) {
	const win = document.getElementById(id);
	if (!win) return;

	bringToFront(id);

	dragInfo = {
		win,
		offsetX: event.clientX - win.offsetLeft,
		offsetY: event.clientY - win.offsetTop
	};

	document.addEventListener("mousemove", handleDrag);
	document.addEventListener("mouseup", stopDrag);
}

function handleDrag(event) {
	if (!dragInfo) return;
	const { win, offsetX, offsetY } = dragInfo;
	win.style.left = `${event.clientX - offsetX}px`;
	win.style.top = `${event.clientY - offsetY}px`;
}

function stopDrag() {
	document.removeEventListener("mousemove", handleDrag);
	document.removeEventListener("mouseup", stopDrag);
	dragInfo = null;
}

// --- Resize ---
let resizeInfo = null;

function startResize(event, id, direction) {
	const win = document.getElementById(id);
	if (!win) return;

	bringToFront(id);

	resizeInfo = {
		win,
		direction,
		startX: event.clientX,
		startY: event.clientY,
		startWidth: win.offsetWidth,
		startHeight: win.offsetHeight,
		startLeft: win.offsetLeft,
		startTop: win.offsetTop
	};

	document.addEventListener("mousemove", handleResize);
	document.addEventListener("mouseup", stopResize);
}

function handleResize(event) {
	if (!resizeInfo) return;
	const { win, direction, startX, startY, startWidth, startHeight, startLeft, startTop } = resizeInfo;
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

function stopResize() {
	document.removeEventListener("mousemove", handleResize);
	document.removeEventListener("mouseup", stopResize);
	resizeInfo = null;
}

function maximizeWindow(id) {
    const win = document.getElementById(id);
    if (!win) return;

    if (win.dataset.maximized === "true") {
        // restore
        win.style.left = win.dataset.left;
        win.style.top = win.dataset.top;
        win.style.width = win.dataset.width;
        win.style.height = win.dataset.height;
        win.dataset.maximized = "false";
    } else {
        // save original
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

function closeWindow(id) {
    const win = document.getElementById(id);
    if (!win) return;

    win.style.display = "none";

    const taskItem = document.querySelector(`.taskbar-item[data-window="${id}"]`);
    if (taskItem) taskItem.classList.remove("active");
}
