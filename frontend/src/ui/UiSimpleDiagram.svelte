<script lang="typescript">
import { onMount } from "svelte";

	import { on } from "../util";

	export let min = 0;
	export let max: number;
	export let count = 100;
	export let fillStyle: string | CanvasGradient | CanvasPattern | undefined = undefined;
	// [height, description, color]
	export let lines: [number, string, string | CanvasGradient | CanvasPattern][] = [];

	export let width: number | undefined = undefined;
	export let height: number | undefined = undefined;
	export let style: string | undefined = undefined;

	let history: number[] = [];

	let canvas: HTMLCanvasElement;
	let ctx: CanvasRenderingContext2D | null = null;

	export function addValue(val: number) {
		if (history.length > count)
			history = [...history.slice(history.length - count), val];
		else
			history.push(val);
		redraw();
	}

	export function clear() {
		history = [];
		redraw();
	}

	$: cutDown(count);

	$: on(min, max, fillStyle, lines, width, height, redraw());

	function cutDown(count: number) {
		if (history.length > count) {
			history = history.slice(history.length - count);
			redraw();
		}
	}

	function redraw() {
		if (ctx === null) return;
		// Get size from component if not set
		const realWidth = width ?? canvas.width;
		const realHeight = height ?? canvas.height;
		const X_STEP = realWidth / count;
		const Y_STEP = realHeight / (max - min);

		function getX(val: number): number {
			return X_STEP * (val + count - history.length);
		}

		function getY(val: number): number {
			return realHeight - Y_STEP * (val - min);
		}

		ctx.clearRect(0, 0, realWidth, realHeight);
		if (history.length !== 0) {
			ctx.beginPath();
			ctx.moveTo(getX(0), realHeight);
			ctx.lineTo(getX(0), getY(history[0]));

			for (let i = 1; i < history.length - 1; i++)
				ctx.lineTo(getX(i), getY(history[i]));
				//ctx.quadraticCurveTo(getX(i - 1), getY(history[i - 1]), getX(i + 1), getY(history[i + 1]));
			ctx.lineTo(realWidth, getY(history[history.length - 1]));
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
	}

	onMount(() => {
		ctx = canvas.getContext("2d");
		redraw();
	});
</script>

<canvas bind:this={canvas} width={width} height={height} style={style}></canvas>
