<script lang="ts">
	import Chart from "chart.js";
	import { onMount } from "svelte";

	export let config: Chart.ChartConfiguration;
	let chartCanvas: HTMLCanvasElement;
	let chart: Chart;

	export function updateChart(): void {
		chart.update();
	}

	onMount(() => {
		chart = new Chart(chartCanvas, config);
		return () => chart.destroy();
	});
</script>

<canvas bind:this={chartCanvas} />