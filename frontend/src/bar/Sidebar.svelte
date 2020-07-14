<script lang="typescript">
	import Server from "../tree/Server.svelte";
	import ServerIcon from "../ui/ServerIcon.svelte";
	import { Connection } from "../connection";

	export let connection!: Connection;
	export let filter!: string;
	let server = connection.book.server;
	let selectedChat = connection.chat.selectedChat;

	declare let selectedServerChat: boolean;
	$: selectedServerChat = "Server" in $selectedChat;
</script>

<aside class="sidebar">
	<button class="entry-expand button" class:selectedServerChat on:click={() => connection.chat.selectServer()}>
		<ServerIcon {connection} />
		<span class="expand" class:selected-server={true} style={$server.getColor()}>
			{$server.name}
		</span>
	</button>
	<Server {connection} {filter} />

	<button class="entry-expand button chats-header">
		<span class="entry-expand">
			Splamy (maybe)
		</span>
	</button>
	<div class="menu">
		<ul class="menu-list">
			<li>
				<div class="channel-line">
				</div>
				<ul class="menu-list">
					<li>User</li>
				</ul>
			</li>
		</ul>
	</div>
</aside>

<style lang="scss">
	.button {
		background: none;
		border: none;
		border-radius: 0;

		position: sticky;
		bottom: 0;
		top: 0;
		z-index: 10;
		background-color: $background;
		box-shadow: 0 0.3em 0.3em #0005;

		width: 100%;
		justify-content: start;
	}
	.button:focus {
		box-shadow: none;
	}

	.sidebar {
		display: inline-flex;
		flex-direction: column;
		background-color: $box-background-color;
		border-right: rgb(179, 179, 179) 2px solid;
		overflow-y: auto;
	}

	button.selectedServerChat {
		background-color: mix($background, $text, 80%);
	}

	.sidebar button.chats-header {
		top: 2.2em;
	}

	.sidebar > .menu .menu-list li {
		margin: 2em;
	}
</style>
