<svelte:options immutable />

<script lang="ts">
	export let raw: string | undefined = undefined;
	export let size: number | string = "1.2em";
	export let name: string | undefined = undefined;
	export let isLeft: boolean = false;
	export let isRight: boolean = false;
	export let style: string = "";
	export let title: string | undefined = undefined;
</script>

<span class="icon{isLeft ? ' is-left' : ''}{isRight ? ' is-right' : ''}" {title}>
	{#if raw}
		{raw}
	{:else}
		<i
			class="mdi mdi-{name ?? 'dummy'}"
			style="font-size:{typeof size === 'number' ? size + 'px' : size};{style}"
		/>
	{/if}
</span>

<style lang="scss">
	.mdi-dummy::before {
		content: "";
	}
</style>
