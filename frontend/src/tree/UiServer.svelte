<script lang="typescript">
	import StickySlot from "../ui/StickySlot.svelte";
	import ServerName from "../ui/ServerName.svelte";
	import TsIcon from "../ui/TsIcon.svelte";
	import Loader from "../ui/Loader.svelte";
	import UiChannel from "./UiChannelWrap.svelte";
	import { Connection } from "../connection";
	import { ConnectData } from "../connect/connect";
	import { flash, render_updates } from "../util";
	import { afterUpdate } from "svelte";
	import { app } from "../app";

	let div: HTMLElement;
	if (render_updates) afterUpdate(() => flash(div));

	export let connection: Connection;
	export let filter: string;
	export let showConnect: (data: ConnectData) => void;

	const state = connection.state;
	const server = connection.book.server;
	let channels = server.channels;
	$: filterStartFromRoot = filter.includes("/");
	$: selectedServerChat = $server.isSelected;

	function click() {
		if (!$state.connected) {
			showConnect(connection.connectOptions.clone());
		} else {
			app.select(connection, server);
		}
	}

	function cancel() {
		connection.close();
	}
</script>

<StickySlot styled={false} on:click={click}>
	<div bind:this={div} class="button serverHeader" class:selectedServerChat>
		<TsIcon type="server" source={$server} {connection} />
		<ServerName {connection} />
		<div class="buttons">
			{#if !$state.connected}
				<button class="button is-danger is-small" on:click|stopPropagation={cancel}>Cancel</button>
			{/if}
		</div>
		<span class="icons">
			{#if $state.connected && $server.chat.unreadCount > 0}
				<span class="unreadCount" title={$server.chat.unreadCount.toString()}>
					{#if $server.chat.unreadCount >= 100}
						99+
					{:else}
						{$server.chat.unreadCount}
					{/if}
				</span>
			{/if}
		</span>
	</div>
</StickySlot>

{#if !$state.connected}
	<div class="statusField">
		<div class="notification" class:is-danger={$state.errored}>
			{#if $state.errored}
				{$state.error}
			{:else}
				<Loader text={'Connecting ...'} />
			{/if}
		</div>
	</div>
{:else}
	<div class="menu channel-list">
		<ul class="menu-list">
			{#each $channels as channel (channel.id)}
				<UiChannel {connection} {filter} {filterStartFromRoot} {channel} />
			{/each}
		</ul>
	</div>
{/if}

<style lang="scss">
	@import "./tree";

	ul {
		margin: 0 0 0 0.2em;
	}

	:global(.innerContainer.dragStyle) {
		background-color: #6040c080 !important;
		z-index: 3 !important;
	}

	.serverHeader {
		background: transparent;
		border: none;
		border-radius: 0;
		width: 100%;
		justify-content: flex-start;
		display: flex;

		&:focus {
			box-shadow: none;
		}

		&.selectedServerChat {
			background-color: mix($background, $text, 95%);
		}
	}

	// Server name
	.serverHeader :global():nth-child(2) {
		flex: 1;
		text-align: start;
		overflow: hidden;
		text-overflow: ellipsis;
		margin-right: 0.25em;
	}

	.statusField {
		padding: 1em;
	}

	.buttons, .button {
		margin-bottom: 0;
	}
</style>
