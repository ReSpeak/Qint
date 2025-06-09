<script lang="ts">
	import { onMount } from "svelte";
	import { clearMenu, mX, mY } from "../contextMenu";

	let div: HTMLDivElement;

	function onClick(ev: MouseEvent) {
		ev.stopPropagation();
		if ((ev.target as HTMLElement).closest("button, .inlineButton") !== null) clearMenu();
	}

	onMount(() => {
		div.focus();
	});
</script>

<div
	bind:this={div}
	on:click={onClick}
	tabindex="0"
	class="context menu"
	style="left: {mX}px; top: {mY}px;"
>
	<slot />
</div>

<style lang="scss">
	@use "../index.scss" as *;
	@import "../style/global_mixin";

	.context {
		position: fixed;
		z-index: 350;
		border: solid 1px $border;
		background: $background;
		display: flex;
		flex-direction: column;
		gap: 0.3em;
		margin-bottom: 0.3em;
		box-shadow: rgb(0 0 0 / 25%) 3px 3px 6px 4px;
	}

	.context :global(button) {
		background: none;
		border: none;
		color: $text;
		text-align: start;
		display: flex;
		align-items: center;
		gap: 0.5em;
	}

	.context :global(button:hover) {
		background-color: $highlight-weak;
	}
</style>
