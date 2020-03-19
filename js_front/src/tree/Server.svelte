<script>
	import Channel from "./Channel";

	export let connection;
	let selected_server = true; // TODO
	let icon = ""; // TODO
</script>

<div class="menu channel-list">
	<p class="menu-label" onclick="set_chat">
		{icon}
		<span class="expand" class:selected-server="{selected_server}">
			{connection.server.name}
		</span>
	</p>
	<ul class="menu-list">
		{#each connection.channel as channel}
			<Channel/>
		{/each}
	</ul>
</div>
