<script lang="typescript">
	export let min!: number;
	export let max!: number;
	export let step!: number;
	export let value!: number;
	export let tooltip!: boolean;

	let slider!: HTMLElement;
	let tooltip_left = 0;
	$: if (tooltip && slider) {
		const perc = (value - min) / (min - max);
		tooltip_left = -((slider.clientWidth - 48 / 2) * perc) - (48 / 2);
	}
</script>

<div class="bslider">
	<input
		bind:this="{slider}"
		type="range"
		class="slider is-fullwidth"
		class:has-output-tooltip="{tooltip}"
		{min}
		{max}
		bind:value
		{step}
	/>
	{#if tooltip}
		<output style="left:calc({tooltip_left}px + 0.5rem);">{value}</output>
	{/if}
</div>

<style>
	.bslider {
		position: relative;
		padding-top: 1em;
	}
</style>
