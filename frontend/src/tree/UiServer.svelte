<script lang="typescript">
	import UiChannel from "./UiChannel.svelte";
	import { Channel } from "./book";
	import { Connection } from "../connection";

	export let connection: Connection;
	export let filter: string;
	let book = connection.book;
	let server = book.server;
	let children = $server.children;
</script>

<div class="menu channel-list">
	<ul class="menu-list">
		{#each $children as channel (channel.key)}
			{#if channel instanceof Channel}
				<UiChannel {connection} {filter} {channel} />
			{:else}
				{@debug channel}
			{/if}
		{/each}
	</ul>
</div>

<style lang="scss">
	ul {
		margin: 0 0 0 0.2em;
	}

	:global(.dragStyle) {
		background-color: #6040c080 !important;
	}
</style>
