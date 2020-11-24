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
	public mouseStart!: MouseEvent;
	public mouseMove!: MouseEvent;
	public mouseDrop!: MouseEvent;
	public customData: any | undefined;
	public customDragNode?: HTMLElement

	constructor(
		public dragNode: HTMLElement,
		public enabled: boolean
	) { }
}

export function draggable(node: HTMLElement, enabled: boolean = true) {
	let dd = new DragData(node, enabled);

	function handleMousedown(event: MouseEvent) {
		if (!dd.enabled) return;
		if (event.button !== MouseButton.Main) return;
		dd.mouseStart = event;
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

		dd.mouseMove = event;
		if (!dd.hasTriggered) {
			if (Math.abs(dx) + Math.abs(dy) < dd.minDistBeforeTrigger)
				return;
			dd.hasTriggered = true;
			node.style.pointerEvents = "none";
			node.dispatchEvent(new CustomEvent('svddrag', { detail: dd }));
		} else {
			node.dispatchEvent(new CustomEvent('svdmove', { detail: dd }));
			dd.dragNode.style.transform = `translate(${dx}px,${dy}px)`;
		}
	}

	function handleMouseup(event: MouseEvent) {
		dd.mouseDrop = event;
		stopDrag();
	}

	function stopDrag() {
		window.removeEventListener('mousemove', handleMousemove);
		window.removeEventListener('mouseup', handleMouseup);
		if (dd.hasTriggered) {
			dd.dragNode.style.transform = null!;
			node.style.pointerEvents = null!;
			node.dispatchEvent(new CustomEvent('svddrop', { detail: dd }));
		}
	}

	node.addEventListener('mousedown', handleMousedown);

	return {
		destroy() {
			node.removeEventListener('mousedown', handleMousedown);
			stopDrag();
		},
		update(enabled: boolean) {
			dd.enabled = enabled;
			if (!enabled)
				stopDrag();
		}
	};
}

export const enum MouseButton {
	/** usually the left button or the un-initialized state */
	Main = 0,
	/** usually the wheel button or the middle button (if present) */
	Auxiliary = 1,
	/** usually the right button */
	Secondary = 2,
	/** typically the Browser Back button */
	Fourth = 3,
	/** typically the Browser Forward button */
	Fifth = 4,
}
