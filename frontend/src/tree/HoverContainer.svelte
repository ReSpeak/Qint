<script lang="ts">
	import { createEventDispatcher } from "svelte";
	import Icon from "../ui/Icon.svelte";

	export let div: HTMLElement;
	export let closeButton: boolean = false;

	const dispatch = createEventDispatcher<{ close: undefined }>();

	function close() {
		dispatch("close");
	}
</script>

<div class="hover menu" style="top: calc({div.getBoundingClientRect().top}px - 1.5em);">
	<div class="corner" />
	{#if closeButton}
		<button
			class="toolbutton closeButton"
			on:click={close}>
			<Icon name="close" />
		</button>
	{/if}
	<slot />
</div>

<style lang="scss">
	.hover {
		position: fixed;
		z-index: 20;
		border: solid 1px $border;
		border-radius: 0.5em;
		background: $background;
		padding: 0.5em;
		left: var(--channel-tree-width);
		display: flex;
		flex-direction: column;
		align-items: flex-start;
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
