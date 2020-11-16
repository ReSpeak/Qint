<script lang="typescript">
	import Chart from "chart.js";
	import { onMount } from "svelte";
	import "chartjs-plugin-streaming";

	export let config: Chart.ChartConfiguration;
	let chartCanvas: HTMLCanvasElement;

	onMount(() => {
		const chart = new Chart(chartCanvas, config);
		return () => chart.destroy();
	});
</script>

<canvas bind:this={chartCanvas} />