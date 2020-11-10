<script lang="typescript">
	import Chart from "chart.js";
import { onMount } from "svelte";

	let chartCanvas: HTMLCanvasElement;

	let datapoints = [0, 20, 20, 60, 60, 120, NaN, 180, 120, 125, 105, 110, 170];
	let config: Chart.ChartConfiguration = {
		type: 'line',
		data: {
			labels: ['0', '1', '2', '3', '4', '5', '6', '7', '8', '9', '10', '11', '12'],
			datasets: [{
				label: 'Packet loss (out)',
				data: datapoints,
				borderColor: "red",
				backgroundColor: 'rgba(0, 0, 0, 0)',
				fill: false,
				cubicInterpolationMode: 'monotone'
			}, {
				label: 'Packet loss (in)',
				data: datapoints,
				borderColor: "blue",
				backgroundColor: 'rgba(0, 0, 0, 0)',
				fill: false,
			}, {
				label: 'Linear interpolation',
				data: datapoints,
				borderColor: "white",
				backgroundColor: 'rgba(0, 0, 0, 0)',
				fill: false,
				lineTension: 0
			}]
		},
		options: {
			responsive: true,
			title: {
				display: true,
				text: 'Chart.js Line Chart - Cubic interpolation mode'
			},
			tooltips: {
				mode: 'index'
			},
			scales: {
				xAxes: [{
					display: true,
					scaleLabel: {
						display: true
					}
				}],
				yAxes: [{
					display: true,
					scaleLabel: {
						display: true,
						labelString: 'Value'
					},
					ticks: {
						suggestedMin: -10,
						suggestedMax: 200,
					}
				}]
			}
		}
	};

	onMount(() => {
		new Chart(chartCanvas, config);
	});
</script>

<canvas bind:this={chartCanvas} width="200" height="150" />