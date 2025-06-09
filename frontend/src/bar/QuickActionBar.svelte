<script lang="ts">
	import QuickActionButtons from "./QuickActionButtons.svelte";
	import { app } from "../app";

	const selections = app.selectedNode;
	$: selected = $selections.getSingleSelection();
</script>

<div class="actionBar inlineButtons">
	{#if selected !== undefined}
		<QuickActionButtons {selected} />
	{/if}
</div>

<style lang="scss">
	@use "../index.scss" as *;
	@import "../style/global_mixin";

	.actionBar {
		background-color: $box-background-color;
		flex: unset !important; // TODO nicer
		width: 2em;
		z-index: 6;
		box-shadow: 3px 0px 5px black;

		flex-direction: column;
	}
</style>
