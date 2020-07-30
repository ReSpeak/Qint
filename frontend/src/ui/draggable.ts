export class DragData {
	public x!: number;
	public y!: number;
	public lockX: boolean = false;
	public lockY: boolean = false;
	public minX?: number;
	public maxX?: number;
	public minY?: number;
	public maxY?: number;
	public minDistBeforeTrigger = 5;
	public hasTriggered = false;
	public mouseEvent!: MouseEvent;
	public customData: any | undefined;

	constructor(
		public dragNode: HTMLElement
	) { }
}

export function draggable(node: HTMLElement) {
	let dd = new DragData(node);

	function handleMousedown(event: MouseEvent) {
		dd.x = event.clientX;
		dd.y = event.clientY;
		dd.hasTriggered = false;
		window.addEventListener('mousemove', handleMousemove);
		window.addEventListener('mouseup', handleMouseup);
	}

	function handleMousemove(event: MouseEvent) {
		let dx, dy;
		if (dd.lockX) dx = 0;
		else {
			dx = event.clientX - dd.x;
			if (dd.minX !== undefined) dx = Math.max(dd.minX, dx);
			if (dd.maxX !== undefined) dx = Math.min(dd.maxX, dx);
		}
		if (dd.lockY) dy = 0;
		else {
			dy = event.clientY - dd.y;
			if (dd.minY !== undefined) dy = Math.max(dd.minY, dy);
			if (dd.maxY !== undefined) dy = Math.min(dd.maxY, dy);
		}

		dd.mouseEvent = event;
		if (!dd.hasTriggered) {
			if (Math.abs(dx) + Math.abs(dy) < dd.minDistBeforeTrigger)
				return;
			dd.hasTriggered = true;
			node.style.pointerEvents = "none";
			node.dispatchEvent(new CustomEvent('svddrag', { detail: dd }));
		} else {
			node.dispatchEvent(new CustomEvent('svdmove', { detail: dd }));
		}
		node.style.transform = `translate(${dx}px,${dy}px)`;
	}

	function handleMouseup(event: MouseEvent) {
		dd.mouseEvent = event;
		if (dd.hasTriggered) {
			node.style.transform = `translate(0,0)`;
			node.style.pointerEvents = "unset";
			node.dispatchEvent(new CustomEvent('svddrop', { detail: dd }));
		}
		window.removeEventListener('mousemove', handleMousemove);
		window.removeEventListener('mouseup', handleMouseup);
	}

	node.addEventListener('mousedown', handleMousedown);

	return {
		destroy() {
			node.removeEventListener('mousedown', handleMousedown);
		}
	};
}
