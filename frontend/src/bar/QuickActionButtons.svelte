<script lang="ts">
	import Icon from "../ui/icon/Icon.svelte";
	import { app, NodeSelection, NodeSelections } from "../app";
	import { DescriptionMode } from "../transientSettings";
	import { Client } from "../book";

	export let selected: NodeSelection;

	const curSelected = app.selectedNode;
	const descriptionMode = app.transientSettings.ui._descriptionMode;

	let infoActive: boolean;
	let editActive: boolean;
	let filesActive: boolean;
	$: {
		if ($curSelected.includes(selected)) {
			infoActive = $descriptionMode === DescriptionMode.Info;
			editActive = $descriptionMode === DescriptionMode.Edit;
			filesActive = $descriptionMode === DescriptionMode.Files;
		} else {
			infoActive = false;
			editActive = false;
			filesActive = false;
		}
	}

	function setDescriptionMode(mode: DescriptionMode) {
		if (
			(mode === DescriptionMode.Info && infoActive) ||
			(mode === DescriptionMode.Edit && editActive) ||
			(mode === DescriptionMode.Files && filesActive)
		) {
			$descriptionMode = DescriptionMode.None;
		} else {
			$descriptionMode = mode;
			app.selectNode(new NodeSelections([selected]));
		}
		app.transientSettings.save();
	}
</script>

<div
	class="inlineButton"
	class:active={infoActive}
	on:click={() => setDescriptionMode(DescriptionMode.Info)}
	title="Details">
	<Icon name="information-outline" />
</div>
<div
	class="inlineButton"
	class:active={editActive}
	on:click={() => setDescriptionMode(DescriptionMode.Edit)}
	title="Edit">
	<Icon name="pencil" />
</div>
{#if !(selected.node instanceof Client)}
	<div
		class="inlineButton"
		class:active={filesActive}
		on:click={() => setDescriptionMode(DescriptionMode.Files)}
		title="Browse files">
		<Icon name="folder" />
	</div>
{/if}

<style lang="scss">
</style>
