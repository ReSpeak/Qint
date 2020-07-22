<script lang="typescript">
	export let min!: number;
	export let max!: number;
	export let step: number = 1;
	export let value!: number;
	export let tooltip: boolean = false;
	export let display: (n: number) => string = n => String(n);

	let slider!: HTMLElement;
	let tooltip_left = 0;
	$: if (tooltip && slider) {
		const perc = (value - min) / (min - max);
		tooltip_left = -((slider.clientWidth - 48 / 2) * perc) - 48 / 2;
	}
</script>

<div class="bslider">
	<input
		bind:this={slider}
		type="range"
		class="slider is-fullwidth"
		class:has-output-tooltip={tooltip}
		{min}
		{max}
		bind:value
		{step}
		on:input />
	{#if tooltip}
		<output style="left:calc({tooltip_left}px + 0.5rem);">{display(value)}</output>
	{/if}
</div>

<style lang="scss">
	.bslider {
		position: relative;
		margin: 0 1em;
	}

	output {
		top: -0.5em !important;
		width: auto !important;
		line-height: 0.5rem !important;
	}
</style>
