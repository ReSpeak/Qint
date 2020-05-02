<script>
	import Channel from "./Channel.svelte";

	export let connection;
	export let filter;
	let book = connection.book;
	let server = book.server;
	let children = $server.children;
	let selected_server = true; // TODO
	let icon = ""; // TODO
</script>

<div class="menu channel-list">
	<ul class="menu-list">
		{#each $children as channel}
			<Channel {connection} {filter} {channel}/>
		{/each}
	</ul>
</div>

<style lang="scss">
	ul {
		margin: 0 0 0 0.2em;
	}
</style>