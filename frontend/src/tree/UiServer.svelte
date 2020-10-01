<script lang="typescript">
	import StickySlot from "../ui/StickySlot.svelte";
	import ServerName from "../ui/ServerName.svelte";
	import TsIcon from "../ui/TsIcon.svelte";
	import Loader from "../ui/Loader.svelte";
	import UiChannel from "./UiChannelWrap.svelte";
	import { Connection } from "../connection";
	import { flash, render_updates } from "../util";
	import { afterUpdate } from "svelte";
	import { app } from "../app";

	let div: HTMLElement;
	if (render_updates) afterUpdate(() => flash(div));

	export let connection: Connection;
	export let filter: string;

	const state = connection.state;
	const server = connection.book.server;
	let channels = server.channels;
	$: filterStartFromRoot = filter.includes("/");
	$: selectedServerChat = $server.isSelected;

	function cancel() {
		connection.close();
	}

	function retry() {}
</script>

<StickySlot styled={false} on:click={() => app.select(connection, server)}>
	<div bind:this={div} class="button serverHeader" class:selectedServerChat>
		<TsIcon type="server" source={$server} {connection} />
		<ServerName {connection} />
	</div>
</StickySlot>

{#if !$state.connected}
	<div class="statusField">
		<div class="buttons">
			<div class="button is-danger" style="flex: 1;" on:click={cancel}>Cancel</div>
			<div
				class="button is-info"
				style="flex: 1;visibility: {$state.errored ? 'visible' : 'hidden'};"
				on:click={retry}>
				Retry
			</div>
		</div>
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

		&:focus {
			box-shadow: none;
		}

		&.selectedServerChat {
			background-color: mix($background, $text, 95%);
		}
	}

	.statusField {
		padding: 1em;
	}
</style>
