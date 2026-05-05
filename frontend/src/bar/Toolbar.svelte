<script lang="ts">
	import Icon from "../ui/icon/Icon.svelte";
	import { DisplayPanel } from "../panel/panel";
	import { IS_TAURI, isMobile } from "../util";
	import { app, NodeSelections } from "../app";
	import MuteButtons from "./MuteButtons.svelte";
	import { ConnectData } from "../connect/uiConnect";
	import Searchbar from "./Searchbar.svelte";
	import { getCurrentWindow } from "@tauri-apps/api/window";
	import { TitleBarStyle } from "../settings";
	import TitleButtons from "./TitleButtons.svelte";
	import { get } from "svelte/store";

	export let displayPanel: DisplayPanel;
	export let showSidebar: boolean;
	export let connectData: ConnectData;
	export let filter: string; // from the search

	const appSettings = app.settings.app;
	const titleBarStyle = appSettings._titleBarStyle;

	function toggleSidebar(show: boolean) {
		showSidebar = show;
	}

	function togglePanel(panel: DisplayPanel) {
		if (displayPanel !== panel) {
			displayPanel = panel;
		} else {
			if (get(app.connections).length === 0) {
				displayPanel = DisplayPanel.Connect;
			} else {
				displayPanel = DisplayPanel.Main;
			}
		}
	}

	const selectedNode = app.selectedNode;
	$: selectedNodeChanged($selectedNode);
	function selectedNodeChanged(node: NodeSelections) {
		if (node.selections.length !== 0) displayPanel = DisplayPanel.Main;
	}

	$: {
		if (IS_TAURI) {
			if ($titleBarStyle === TitleBarStyle.Native) {
				getCurrentWindow().setDecorations(true);
			} else {
				getCurrentWindow().setDecorations(false);
			}
		}
	}
</script>

<div
	class="toolbar"
	class:normalStyle={false}
	class:tinyStyle={true}
	class:bigDesign={false}
	class:smallDesign={true}
>
	<div class="leftBlock">
		<div class="inlineButtons">
			<div
				class="inlineButton"
				class:active={showSidebar}
				on:click={() => toggleSidebar(!showSidebar)}
				title="Channel tree"
			>
				<Icon name="file-tree" />
			</div>
			<div
				class="inlineButton"
				class:active={displayPanel === DisplayPanel.Connect}
				on:click={() => togglePanel(DisplayPanel.Connect)}
				title="Connect to a new server"
			>
				<Icon name="plus" />
			</div>
		</div>
		<div class="flex1" data-tauri-drag-region />
		<MuteButtons bind:connectData />
	</div>
	<div class="flex1" data-tauri-drag-region />
	<div class="inlineButtons">
		<Searchbar bind:filter visible={true} />
		<div class="dragSpace" data-tauri-drag-region />
		<div
			class="inlineButton"
			class:active={displayPanel === DisplayPanel.Settings}
			on:click={() => togglePanel(DisplayPanel.Settings)}
			title="Settings"
		>
			<Icon name="cog" />
		</div>
	</div>

	{#if IS_TAURI && !isMobile() && $titleBarStyle !== TitleBarStyle.Native}
		<TitleButtons />
	{/if}
</div>

<style lang="scss">
	@use "../index.scss" as *;
	@import "../style/global_mixin";

	.toolbar {
		background-color: $box-background-color;
		padding: 0.5em;
		display: flex;

		&.normalStyle {
			padding-top: 0;
		}

		&.tinyStyle {
			padding: 0;
			height: 2em;
		}

		//box-sizing: content-box;
		//border-bottom: 1px cyan solid;
		box-shadow: 0 0 5px black;
		z-index: 5;
	}

	.leftBlock {
		display: flex;
		width: var(--channel-tree-width);
	}

	.dragSpace {
		width: 2em;
	}
</style>
