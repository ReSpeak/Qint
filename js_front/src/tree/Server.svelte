<script lang="typescript">
	import Channel from "./Channel.svelte";
	import { Connection } from "../connection";

	export let connection!: Connection;
	export let filter!: string;
	let book = connection.book;
	let server = book.server;
	let children = $server.children;
</script>

<div class="menu channel-list">
	<ul class="menu-list">
		{#each $children as channel (channel.id)}
			<Channel {connection} {filter} {channel}/>
		{/each}
	</ul>
</div>

<style lang="scss">
	ul {
		margin: 0 0 0 0.2em;
	}
</style>
