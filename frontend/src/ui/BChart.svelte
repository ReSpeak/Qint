<script lang="typescript">
	import Chart from "chart.js";
	import { onMount } from "svelte";
	import "chartjs-plugin-streaming";

	export let config: Chart.ChartConfiguration;
	let chartCanvas: HTMLCanvasElement;

	onMount(() => {
		new Chart(chartCanvas, config);
	});
</script>

<canvas bind:this={chartCanvas} />