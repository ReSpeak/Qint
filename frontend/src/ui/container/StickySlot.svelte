<script lang="ts">
	import { onMount, onDestroy } from "svelte";
	import { getContext } from "svelte";
	export let styled: boolean = true;

	const stickyChanged = getContext("stickyChanged") as () => void;
	if (stickyChanged) {
		onMount(stickyChanged);
		onDestroy(stickyChanged);
	}
</script>

<div on:click on:auxclick class:button={styled} class="stickySlot">
	<slot />
</div>
<div class="stickyDummy" />

<style lang="scss">
	@use "../../index.scss" as *;

	.stickySlot {
		z-index: 300;
		display: block;
		position: sticky;
		top: 0;
		background-color: $background;
		border-radius: 0;
		box-shadow: 0 0.3em 0.3em #0005;
		border-top: 2px solid #425575;
		font-weight: bold;
		text-align: left;
	}
</style>
