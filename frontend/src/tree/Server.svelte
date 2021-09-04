<script lang="ts">
	import StickySlot from "../ui/container/StickySlot.svelte";
	import ServerName from "../ui/name/ServerName.svelte";
	import TsIcon from "../ui/icon/TsIcon.svelte";
	import Loader from "../ui/icon/Loader.svelte";
	import UiChannel from "./ChannelWrap.svelte";
	import { Connection } from "../connection";
	import { ConnectData } from "../connect/uiConnect";
	import { flash, render_updates } from "../util";
	import { afterUpdate } from "svelte";
	import { get } from "svelte/store";
	import { app, NodeSelection } from "../app";
	import ContextMenuServer from "./ContextMenuServer.svelte";
	import ContextMenuContainer from "./ContextMenuContainer.svelte";
	import ChangeResult from "../ui/specialized/ChangeResult.svelte";
	import { DescriptionMode } from "../transientSettings";
	import { MouseButton } from "../ui/util/draggable";
	import { showContextMenu } from "../contextMenu";

	let div: HTMLElement;
	if (render_updates) afterUpdate(() => flash(div));

	export let connection: Connection;
	export let filter: string;
	export let showConnect: (data: ConnectData) => void;

	let contextMenuVisible = false;
	const state = connection.state;
	const server = connection.book.server;
	const channels = server.channels;
	$: chat = server.chat;
	$: filterStartFromRoot = filter.includes("/");
	$: selectedServerChat = $server.isSelected;

	function click(ev: MouseEvent) {
		if (!$state.connected) {
			showConnect(get(connection.connectOptions).clone());
		} else if (ev.button === MouseButton.Main) {
			if (ev.ctrlKey) {
				app.toggleSelection(new NodeSelection(connection, server));
			} else if (ev.shiftKey) {
				app.expandSelection(new NodeSelection(connection, server));
			} else {
				app.setDescriptionMode(new NodeSelection(connection, server), DescriptionMode.Info);
			}
		} else if (ev.button === MouseButton.Auxiliary) {
			ev.preventDefault();
			app.setDescriptionMode(new NodeSelection(connection, server), DescriptionMode.Files);
		}
	}

	function cancel() {
		connection.close();
	}

	function onContextMenu(e: MouseEvent) {
		if (!contextMenuVisible) {
			showContextMenu(e, () => (contextMenuVisible = false));
			contextMenuVisible = true;
		}
	}
</script>

{#if contextMenuVisible}
	<ContextMenuContainer>
		<ContextMenuServer {connection} />
	</ContextMenuContainer>
{/if}
<StickySlot styled={false} on:click={click} on:auxclick={click}>
	<div
		bind:this={div}
		class="button stickyLine"
		class:selectedServerChat
		tabindex="0"
		on:contextmenu={onContextMenu}>
		<TsIcon type="server" source={$server} {connection} />
		<div class="serverName">
			<ServerName server={$server} {connection} handleClicks={false} />
		</div>
		<div class="buttons">
			{#if !$state.connected}
				<button class="button is-danger is-small" on:click|stopPropagation={cancel}
					>Cancel</button>
			{/if}
		</div>
		<span class="icons">
			{#if $state.connected && $chat.unreadCount > 0}
				<span class="unreadCount" title={$chat.unreadCount.toString()}>
					{#if $chat.unreadCount >= 100}99+{:else}{$chat.unreadCount}{/if}
				</span>
			{/if}
		</span>
	</div>
</StickySlot>

{#if !$state.connected}
	<div class="statusField">
		<div class="notification" class:is-danger={$state.errored}>
			{#if $state.errored}
				{#if typeof $state.error === "string"}
					{$state.error}
				{:else if $state.error !== undefined}
					<ChangeResult result={$state.error} />
				{/if}
			{:else}
				<Loader text={"Connecting ..."} />
			{/if}
		</div>
	</div>
{:else}
	<div class="menu channel-list" data-type="connection" data-key={connection.backend.id}>
		<ul class="menu-list">
			{#each $channels as channel (channel.id)}
				<UiChannel {connection} {filter} {filterStartFromRoot} {channel} />
			{/each}
		</ul>
	</div>
{/if}

<style lang="scss">
	@import "./tree_shared";

	ul {
		margin: 0 0 0 0.2em;
	}

	:global(.innerContainer.dragStyle) {
		background-color: #6040c080 !important;
		z-index: 3 !important;
	}

	.stickyLine {
		&.selectedServerChat {
			background-color: mix($background, $text, 95%);
		}

		.serverName {
			flex: 1;
			text-align: start;
			overflow: hidden;
			text-overflow: ellipsis;
			margin-right: 0.25em;
		}
	}

	.statusField {
		padding: 1em;
	}

	.buttons,
	.button {
		margin-bottom: 0 !important;
	}
</style>
