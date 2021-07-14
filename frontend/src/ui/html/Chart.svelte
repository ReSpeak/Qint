<script lang="ts">
	import { onMount } from "svelte";
	import {
		Chart,
		ArcElement,
		LineElement,
		BarElement,
		PointElement,
		BarController,
		LineController,
		ScatterController,
		CategoryScale,
		LinearScale,
		TimeScale,
		TimeSeriesScale,
		Filler,
		Legend,
		Title,
		Tooltip,
	} from "chart.js";
	import "chartjs-adapter-moment";

	Chart.register(
		ArcElement,
		LineElement,
		BarElement,
		PointElement,
		BarController,
		LineController,
		ScatterController,
		CategoryScale,
		LinearScale,
		TimeScale,
		TimeSeriesScale,
		Filler,
		Legend,
		Title,
		Tooltip
	);

	export let config: any;
	let chartCanvas: HTMLCanvasElement;
	let chart: Chart;

	export function updateChart(): void {
		// chartjs does not always update everything when using animations,
		// so disable them
		chart.update("none");
	}

	onMount(() => {
		chart = new Chart(chartCanvas, config);
		return () => chart.destroy();
	});
</script>

<canvas bind:this={chartCanvas} />
