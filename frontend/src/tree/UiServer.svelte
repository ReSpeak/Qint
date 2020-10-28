<script lang="typescript">
	import type { Writable } from "svelte/store";
	import StickySlot from "../ui/StickySlot.svelte";
	import ServerName from "../ui/ServerName.svelte";
	import TsIcon from "../ui/TsIcon.svelte";
	import Loader from "../ui/Loader.svelte";
	import UiChannel from "./UiChannelWrap.svelte";
	import { Connection } from "../connection";
	import { ConnectData } from "../connect/connect";
	import { flash, render_updates } from "../util";
	import { afterUpdate, onMount } from "svelte";
	import { app, NodeSelection } from "../app";
	import HoverMenu from "./HoverMenu.svelte";
	import { DelayedHover } from "./delayedHover";

	let div: HTMLElement;
	if (render_updates) afterUpdate(() => flash(div));

	export let connection: Connection;
	export let filter: string;
	export let showConnect: (data: ConnectData) => void;

	let hover: DelayedHover;
	let hovered: Writable<boolean>;
	let hoverComp: HTMLElement;
	const state = connection.state;
	const server = connection.book.server;
	let channels = server.channels;
	$: chat = server.chat;
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

	onMount(() => {
		hover = new DelayedHover(div, [div, hoverComp]);
		hovered = hover.hovered;

		return () => hover.unregister();
	});
</script>

<StickySlot styled={false} on:click={click}>
	<div bind:this={div} class="button serverHeader" class:selectedServerChat tabindex="0">
		<TsIcon type="server" source={$server} {connection} />
		<ServerName {connection} />
		<div class="buttons">
			{#if !$state.connected}
				<button
					class="button is-danger is-small"
					on:click|stopPropagation={cancel}>Cancel</button>
			{/if}
		</div>
		<span class="icons">
			{#if $state.connected && $chat.unreadCount > 0}
				<span class="unreadCount" title={$chat.unreadCount.toString()}>
					{#if $chat.unreadCount >= 100}
						99+
					{:else}
						{$chat.unreadCount}
					{/if}
				</span>
			{/if}
		</span>
	</div>
</StickySlot>
<div bind:this={hoverComp}>
{#if $hovered}
	<HoverMenu {div} selected={new NodeSelection(connection, server)} />
{/if}
</div>

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
	@import "./tree_shared";

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
	.serverHeader > :global():nth-child(2) {
		flex: 1;
		text-align: start;
		overflow: hidden;
		text-overflow: ellipsis;
		margin-right: 0.25em;
	}

	.statusField {
		padding: 1em;
	}

	.buttons,
	.button {
		margin-bottom: 0 !important;
	}
</style>
