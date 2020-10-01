<script lang="typescript">
	import StickySlot from "../ui/StickySlot.svelte";
	import ServerName from "../ui/ServerName.svelte";
	import TsIcon from "../ui/TsIcon.svelte";
	import UiChannel from "./UiChannelWrap.svelte";
	import { Connection } from "../connection";
	import { flash } from "../util";
	import { afterUpdate } from "svelte";
	import { app } from "../app";

	let div: HTMLElement;
	afterUpdate(() => flash(div));

	export let connection: Connection;
	export let filter: string;

	const server = connection.book.server;
	let channels = server.channels;
	$: filterStartFromRoot = filter.includes("/");
	$: selectedServerChat = $server.isSelected;
</script>

<StickySlot styled={false} on:click={() => app.select(connection, server)}>
	<div bind:this={div} class="button" class:selectedServerChat>
		<TsIcon type="server" source={$server} {connection} />
		<ServerName server={$server} />
	</div>
</StickySlot>

<div class="menu channel-list">
	<ul class="menu-list">
		{#each $channels as channel (channel.id)}
			<UiChannel {connection} {filter} {filterStartFromRoot} {channel} />
		{/each}
	</ul>
</div>

<style lang="scss">
	ul {
		margin: 0 0 0 0.2em;
	}

	:global(.innerContainer.dragStyle) {
		background-color: #6040c080 !important;
		z-index: 3 !important;
	}

	.selectedServerChat {
		background-color: mix($background, $text, 95%);
	}

	.button {
		background: transparent;
		border: none;
		border-radius: 0;
		width: 100%;
		justify-content: flex-start;

		&:focus {
			box-shadow: none;
		}
	}
</style>
