<svelte:options immutable />
<script lang="typescript">
	export let raw: string | undefined = undefined;
	export let size: number | string = "1.2em";
	export let name: string | undefined = undefined;
	export let isLeft: boolean = false;
	export let style: string = "";
</script>

<span class="icon {isLeft ? 'is-left' : ''}" >
	{#if raw}
		{raw}
	{:else}
		<i class="mdi mdi-{name ?? 'dummy'}" style="font-size:{(typeof size === "number" ? size + "px" : size)};{style}" />
	{/if}
</span>

<style>
	.mdi-dummy::before {
		content: "";
	}

	.icon {
		height: 1.5em;
		width: 1.5em;
	}
</style>
