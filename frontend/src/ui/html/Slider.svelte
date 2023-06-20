<script lang="ts">
	export let min: number;
	export let max: number;
	export let step: number = 1;
	export let value: number;
	export let tooltip: boolean = false;
	export let display: (n: number) => string = (n) => String(n);
	export let id: string | undefined = undefined;
	export let isInline: Boolean = false;

	let slider!: HTMLElement;
	let tooltip_left = 0;
	$: if (tooltip && slider) {
		const perc = (value - min) / (min - max);
		tooltip_left = -((slider.clientWidth - 48 / 4) * perc) - 48 / 2;
	}
</script>

<div class="bslider">
	<input
		bind:this={slider}
		{id}
		type="range"
		class="slider is-fullwidth"
		class:has-output-tooltip={tooltip}
		class:input-inline={isInline}
		{min}
		{max}
		bind:value
		{step}
		on:change
		on:input
	/>
	{#if tooltip}
		<output class:output-inline={isInline} style="left:calc({tooltip_left}px + 0.5rem);">{display(value)}</output>
	{/if}
</div>

<style lang="scss">
	.bslider {
		display: flex;
		position: relative;
		flex-grow: 1;
		margin-right: .5em;
	}

	output {
		top: -0.5em !important;
		width: auto !important;
		line-height: 0.5rem !important;
	}

	.input-inline {
		margin: 0;
		min-height: 0;
	}

	.output-inline {
		top: -2.0em !important;
		display: none;
	}

	.bslider:hover .output-inline {
		display: block;
	}

</style>
