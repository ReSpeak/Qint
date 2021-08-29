<script lang="ts">
	import { createEventDispatcher, onMount } from "svelte";

	export let x: number;
	export let y: number;

	const dispatch = createEventDispatcher<{ close: undefined }>();
	let div: HTMLDivElement;

	function onBlur(e: FocusEvent) {
		// TODO Only works sometimes
		if (!(e.relatedTarget instanceof Node) || !div.contains(e.relatedTarget)) {
			setTimeout(() => dispatch("close"));
		}
	}

	onMount(() => {
		div.focus();
	});
</script>

<div bind:this={div} tabindex="0" on:focusout={onBlur} class="context menu" style="left: {x}px; top: {y}px;">
	<slot />
</div>

<style lang="scss">
	@import "../style/global_mixin";

	.context {
		position: fixed;
		z-index: 350;
		border: solid 1px $border;
		border-radius: 0.5em;
		background: $background;
		padding: 0.5em;
		display: flex;
		flex-direction: column;
		gap: 0.5em;
	}

	.context :global(button) {
		background: none;
		border: none;
		color: $text;
		text-align: start;
	}

	.context :global(button:hover) {
		background-color: $highlight-weak;
	}
</style>
