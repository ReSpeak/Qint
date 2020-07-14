<script lang="typescript">
	export let raw: string | undefined = undefined;
	export let size = 18;
	export let name: string | undefined = undefined;
	export let is_left = false;
	export let style = "";

	$: if (name === undefined) name = "dummy";
</script>

<span class="icon {is_left ? 'is-left' : ''}">
	{#if raw}
		{raw}
	{:else}
		<i class="mdi mdi-{name} mdi-{size}px" {style}></i>
	{/if}
</span>

<style>
	.mdi-dummy::before {
		content: "";
	}
</style>
