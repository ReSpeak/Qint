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

<div bind:this={div} tabindex="0" on:focusout={onBlur} class="hover menu" style="left: {x}px; top: {y}px;">
	<slot />
</div>

<style lang="scss">
	.hover {
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

	.hover .corner {
		position: absolute;
		transform: rotate(45deg);
		left: -0.3em;
		top: 1.8em;
		width: 0.5em;
		height: 0.5em;
		border-left: solid 1px $border;
		border-bottom: solid 1px $border;
		background: $background;
	}

	.closeButton {
		font-size: 0.5em;
		float: right;
	}
</style>
