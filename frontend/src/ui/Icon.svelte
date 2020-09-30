<script lang="typescript">
	export let raw: string | undefined = undefined;
	export let size: number = 18;
	export let name: string | undefined = undefined;
	export let isLeft: boolean = false;
	export let style: string = "";
</script>

<span class="icon {isLeft ? 'is-left' : ''}" style="font-size:{size}px;">
	{#if raw}
		{raw}
	{:else}
		<i class="mdi mdi-{name ?? 'dummy'}" {style} />
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
