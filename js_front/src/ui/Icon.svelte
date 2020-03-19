<script>
	export let raw = undefined;
	export let size = 18;
	export let name = undefined;
	export let is_left = false;

	if (name === undefined) name = "dummy";
</script>

<span class="icon {is_left ? 'is-left' : ''}">
	{#if raw}
		{raw}
	{:else}
		<i class="mdi mdi-{name} mdi-{size}-px"></i>
	{/if}
</span>

<style>
	.mdi-dummy::before {
		content: "";
	}
</style>
