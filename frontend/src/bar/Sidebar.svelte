<script lang="typescript">
	import Server from "../tree/Server.svelte";
	import ServerIcon from "../ui/ServerIcon.svelte";
	import StickyList from "../ui/StickyList.svelte";
	import StickySlot from "../ui/StickySlot.svelte";
	import { Connection } from "../connection";

	export let connection!: Connection;
	export let filter!: string;
	let server = connection.book.server;
	let selectedChat = connection.chat.selectedChat;
	declare let selectedServerChat: boolean;
	$: selectedServerChat = "Server" in $selectedChat;
</script>

<aside class="sidebar">
	<StickyList>
		<StickySlot styled={false} on:click={() => connection.chat.selectServer()}>
			<div class="button" class:selectedServerChat>
				<ServerIcon {connection} />
				<span style={$server.getColor()}>
					{$server.name}
				</span>
			</div>
		</StickySlot>

		<Server {connection} {filter} />

		<StickySlot>Notifications</StickySlot>
		<div class="menu">
			<ul class="menu-list">
				<li>
					<div class="channel-line" />
					<ul class="menu-list">
						<li>Splamy (maybe)</li>
					</ul>
				</li>
			</ul>
		</div>
	</StickyList>
</aside>

<style lang="scss">
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

	.sidebar {
		display: inline-flex;
		flex-direction: column;
		background-color: $box-background-color;
		border-right: rgb(179, 179, 179) 2px solid;
		overflow-y: auto;
	}

	.selectedServerChat {
		background-color: mix($background, $text, 80%);
	}

	.sidebar button.chats-header {
		top: 2.2em;
	}

	.sidebar > .menu .menu-list li {
		margin: 2em;
	}
</style>
