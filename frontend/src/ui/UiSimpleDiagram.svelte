<script lang="ts">
	import { onMount } from "svelte";
	import { LOUDNESS_UPDATE_MS, on } from "../util";

	export let min = 0;
	export let max: number;
	export let count: number;
	export let fillStyle: string | CanvasGradient | CanvasPattern | undefined = undefined;
	// [height, description, color]
	export let lines: [number, string, string | CanvasGradient | CanvasPattern][] = [];

	export let width: number | undefined = undefined;
	export let height: number | undefined = undefined;
	export let style: string | undefined = undefined;

	let canvas: HTMLCanvasElement;
	let ctx: CanvasRenderingContext2D | null = null;

	const framelength = LOUDNESS_UPDATE_MS;
	let historySize = count;
	const bufferSize = 5;
	let silence = min;
	// Ring space
	// [..........,....] % (historySize + bufferSize)
	// \start     \end \end+buffer
	// -> start is shifted right when time passes
	//            -> buffer is filled from end to right
	let history: number[] = [];
	let lastRenderTs: number = 0;
	let start: number = 0;
	let bufferCount: number = 0;
	let lengthWithData: number = 0;
	let needRender: boolean = false;

	$: on(min, max, count, build(), redrawNow());

	function build() {
		historySize = count;
		silence = min;
		history = new Array(historySize + bufferSize).fill(silence);
		start = 0;
		bufferCount = 0;
		lengthWithData = 0;
		needRender = true;
	}

	export function addValue(value: number, timestamp: number): void {
		move(timestamp);
		if (bufferCount < bufferSize) {
			history[(start + historySize + bufferCount) % history.length] = value;
			bufferCount += 1;
			if (lengthWithData === 0) lastRenderTs = timestamp;
			lengthWithData = historySize + bufferCount;
			needRender = true;
		}
	}

	function move(timestamp: number): void {
		if (lengthWithData === 0) return;
		const elapsed = timestamp - lastRenderTs;
		const elapsedFrames = Math.floor(elapsed / framelength);
		if (elapsedFrames > 0) {
			lastRenderTs += elapsedFrames * framelength;
			for (let i = 0; i < Math.min(elapsedFrames, lengthWithData); i++) {
				history[start] = silence;
				start = (start + 1) % history.length;
			}
			bufferCount = Math.max(0, bufferCount - elapsedFrames);
			lengthWithData = Math.max(0, lengthWithData - elapsedFrames);
			if (lengthWithData === 0) {
				start = 0;
			}
			needRender = true;
		}
	}

	// returns true if another redraw for the next frame is requested
	export function redraw(timestamp: number): boolean {
		if (ctx === null) return false;

		move(timestamp);
		if (!needRender) return lengthWithData > 0;
		needRender = false;

		// Get size from component if not set
		const realWidth = width ?? canvas.width;
		const realHeight = height ?? canvas.height;
		const X_STEP = realWidth / historySize;
		const Y_STEP = realHeight / (max - min);

		function getX(val: number): number {
			return Math.floor(X_STEP * val);
		}

		function getY(val: number): number {
			return Math.floor(realHeight - Y_STEP * (val - min));
		}

		ctx.clearRect(0, 0, realWidth, realHeight);
		if (lengthWithData > 0) {
			ctx.beginPath();
			ctx.moveTo(getX(0), realHeight);

			let endA, endB;
			const startA = start;
			const end = start + lengthWithData;
			if (end > history.length) {
				endA = history.length;
				endB = end - history.length;
			} else {
				endA = end;
				endB = 0;
			}

			for (let i = startA; i < endA; i++) {
				ctx.lineTo(getX(i - startA), getY(history[i]));
			}
			const off = endA - startA;
			for (let i = 0; i < endB; i++) {
				ctx.lineTo(getX(i + off), getY(history[i]));
			}
			ctx.lineTo(getX(endB + off), getY(min));

			ctx.lineTo(realWidth, realHeight);
			ctx.closePath();
			if (fillStyle !== undefined) {
				ctx.fillStyle = fillStyle;
			} else {
				const gradient = ctx.createLinearGradient(0, realHeight, 0, 0);
				gradient.addColorStop(0, "#00bbbb");
				gradient.addColorStop(0.5, "#bb00bb");
				gradient.addColorStop(1, "#bb0000");
				ctx.fillStyle = gradient;
			}
			ctx.fill();
		}

		for (const l of lines) {
			ctx.fillStyle = l[2];
			const lineY = getY(l[0]);
			ctx.fillRect(0, lineY, realWidth, 0.8);
			ctx.fillText(l[1], 5, lineY - 1);
		}

		return lengthWithData > 0;
	}

	export function redrawNow(): void {
		redraw(performance.now());
	}

	onMount(() => {
		ctx = canvas.getContext("2d");
		if (ctx) ctx.imageSmoothingEnabled = false;
		redrawNow();
	});
</script>

<canvas bind:this={canvas} {width} {height} {style} />
