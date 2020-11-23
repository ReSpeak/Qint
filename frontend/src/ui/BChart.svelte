<script lang="typescript">
	import Chart from "chart.js";
	import { onMount } from "svelte";

	export let config: Chart.ChartConfiguration;
	let chartCanvas: HTMLCanvasElement;

	onMount(() => {
		console.log(config);
		const chart = new Chart(chartCanvas, config);
		return () => chart.destroy();
	});
</script>

<canvas bind:this={chartCanvas} />