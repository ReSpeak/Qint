type CloseFn = () => void;

let currentMenu: CloseFn | null = null;
export let mX: number = 0;
export let mY: number = 0;

export function showContextMenu(ev: MouseEvent, close: CloseFn): void {
	clearMenu();

	ev.preventDefault();
	ev.stopPropagation();
	mX = ev.pageX;
	mY = ev.pageY;

	document.addEventListener("click", handleGlobalClick);
	document.addEventListener("contextmenu", handleGlobalContextmenu);
	document.addEventListener("keydown", handleGlobalKeypress);
	currentMenu = close;
}

export function clearMenu(): void {
	if (currentMenu !== null) {
		console.log("clearing, click");
		currentMenu();
		currentMenu = null;

		document.removeEventListener("click", handleGlobalClick);
		document.removeEventListener("contextmenu", handleGlobalContextmenu);
		document.removeEventListener("keydown", handleGlobalKeypress);
	}
}

function handleGlobalClick(this: Document, _ev: MouseEvent) {
	clearMenu();
}

function handleGlobalContextmenu(this: Document, _ev: MouseEvent) {
	clearMenu();
}

function handleGlobalKeypress(this: Document, ev: KeyboardEvent) {
	if (ev.code === "Escape") {
		clearMenu();
	}
}
