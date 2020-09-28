<script lang="typescript">
	import UiServer from "../tree/UiServer.svelte";
	import ServerName from "../ui/ServerName.svelte";
	import TsIcon from "../ui/TsIcon.svelte";
	import StickyList from "../ui/StickyList.svelte";
	import StickySlot from "../ui/StickySlot.svelte";
	import { Connection } from "../connection";

	export let connection: Connection;
	export let filter: string;
	let server = connection.book.server;
	let selectedChat = connection.chat.selectedChat;
	$: selectedServerChat = "Server" in $selectedChat;
</script>

<aside class="sidebar">
	<StickyList>
		<StickySlot styled={false} on:click={() => connection.chat.selectServer()}>
			<div class="button" class:selectedServerChat>
				<TsIcon type="server" source={$server} {connection} />
				<ServerName server={$server} />
			</div>
		</StickySlot>

		<UiServer {connection} {filter} />

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
		box-shadow: 3px 0 3px #0006;
		overflow-y: auto;
	}

	.selectedServerChat {
		background-color: mix($background, $text, 95%);
	}

	.sidebar > .menu .menu-list li {
		margin: 2em;
	}
</style>
