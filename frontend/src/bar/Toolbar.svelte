<script lang="ts">
	import Icon from "../ui/icon/Icon.svelte";
	import { DisplayPanel } from "../panel/panel";
	import { IS_TAURI, SERVER_ICON } from "../util";
	import { app, NodeSelections } from "../app";
	import ConnectionSettings from "./ConnectionSettings.svelte";
	import { ConnectData } from "../connect/uiConnect";
	import Searchbar from "./Searchbar.svelte";
	import { appWindow } from "@tauri-apps/api/window";
	import { TitleBarStyle } from "../transientSettings";
	import TitleButtons from "./TitleButtons.svelte";

	export let displayPanel: DisplayPanel;
	export let showSidebar: boolean;
	export let connectData: ConnectData;
	export let filter: string; // from the search

	let appSettings = app.transientSettings.app;
	let titleBarStyle = appSettings._titleBarStyle;

	function toggleSidebar(show: boolean) {
		showSidebar = show;
	}

	const selectedNode = app.selectedNode;
	$: selectedNodeChanged($selectedNode);
	function selectedNodeChanged(node: NodeSelections) {
		if (node.selections.length !== 0) displayPanel = DisplayPanel.Main;
	}

	function startDragWindow(this: HTMLElement, ev: MouseEvent) {
		if (IS_TAURI) {
			if ((ev.target as HTMLElement)?.dataset?.titledrag) {
				appWindow.startDragging();
			}
		}
	}

	let supportedStyle: TitleBarStyle;
	$: {
		if (IS_TAURI) {
			supportedStyle = $titleBarStyle;
			if (supportedStyle === TitleBarStyle.Native) {
				appWindow.setDecorations(true);
			} else {
				appWindow.setDecorations(false);
			}
		} else {
			supportedStyle = TitleBarStyle.Native;
		}
	}
</script>

{#if supportedStyle === TitleBarStyle.Normal}
	<div class="titlebar" on:mousedown={startDragWindow}>
		<div class="flex1" data-titledrag="1" />
		<TitleButtons />
	</div>
{/if}

<div
	class="toolbar"
	class:normalStyle={supportedStyle === TitleBarStyle.Normal}
	class:tinyStyle={supportedStyle === TitleBarStyle.Tiny}
	class:bigDesign={supportedStyle !== TitleBarStyle.Tiny}
	class:smallDesign={supportedStyle === TitleBarStyle.Tiny}
	on:mousedown={startDragWindow}>
	<div class="leftButtons">
		<div
			class="hybridTitleButton"
			class:active={showSidebar}
			on:click={() => toggleSidebar(!showSidebar)}
			title="Channel tree">
			<Icon name="file-tree" />
		</div>
		<div class="dragSpace" data-titledrag="1" />
		<Searchbar bind:filter visible={true} />
	</div>
	<div class="flex1" data-titledrag="1" />
	<div class="centerButtons hybridTitleButtons">
		{#if filter !== ""}
			<div
				class="hybridTitleButton"
				class:active={displayPanel === DisplayPanel.Search}
				on:click={() => (displayPanel = DisplayPanel.Search)}
				title="Chat">
				<Icon name="magnify" />
			</div>
		{/if}
		<div
			class="hybridTitleButton"
			class:active={displayPanel === DisplayPanel.Main}
			on:click={() => (displayPanel = DisplayPanel.Main)}
			title="Chat">
			<Icon name="chat-outline" />
		</div>
		<div
			class="hybridTitleButton"
			class:active={displayPanel === DisplayPanel.Settings}
			on:click={() => (displayPanel = DisplayPanel.Settings)}
			title="Settings">
			<Icon name="cog" />
		</div>
		<div
			class="hybridTitleButton"
			class:active={displayPanel === DisplayPanel.Connect}
			on:click={() => (displayPanel = DisplayPanel.Connect)}
			title="Connect to a new server">
			<Icon name={SERVER_ICON} />
		</div>
	</div>
	<div class="flex1" data-titledrag="1" />
	<div class="rightButtons">
		<ConnectionSettings bind:connectData />
	</div>

	{#if supportedStyle === TitleBarStyle.Compact || supportedStyle === TitleBarStyle.Tiny}
		<div class="flex1" data-titledrag="1" />
		<TitleButtons />
	{/if}
</div>

<style lang="scss">
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
	}

	.centerButtons,
	.leftButtons,
	.rightButtons {
		display: inline-flex;
	}

	.dragSpace {
		width: 2em;
	}

	.titlebar {
		height: 1.5em;
		display: flex;

		background-color: $box-background-color;
	}
</style>
