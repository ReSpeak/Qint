<script>
	import Channel from "./Channel.svelte";

	export let connection;
	let book = connection.book;
	let server = book.server;
	let children = $server.children;
	console.log($children);
	let selected_server = true; // TODO
	let icon = ""; // TODO
</script>

<div class="menu channel-list">
	<p class="menu-label">
		{icon}
		<span class="expand" class:selected-server="{selected_server}">
			{$server.name}
		</span>
	</p>
	<ul class="menu-list">
		{#each $children as channel}
			<Channel {channel}/>
		{/each}
	</ul>
</div>
