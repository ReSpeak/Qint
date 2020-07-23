<script lang="typescript">
	export let raw: string | undefined = undefined;
	export let size: number = 18;
	export let name: string | undefined = undefined;
	export let isLeft: boolean = false;
	export let style: string = "";

	$: if (name === undefined) name = "dummy";
</script>

<span class="icon {isLeft ? 'is-left' : ''}">
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
