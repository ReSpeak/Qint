<script>
	import { writable } from "svelte/store";
	import Server from "../tree/Server.svelte";
	import ServerIcon from "../ui/ServerIcon.svelte";

	export let connection;
	let server = connection.book.server;
	let selectedChat = connection.chat.selectedChat;
	let dropdownActive = false;
	let dropdown;
	let searchValue = "";
	let filter = writable("");

	$: filter.set(searchValue);
	$: selectedServerChat = "Server" in $selectedChat;

	function handleFocus(event) {
		// Check of the target lies within the dropdown
		if (event.relatedTarget) {
			if (!dropdown.contains(event.relatedTarget)) {
				dropdownActive = false;
			}
		}
	}
</script>

<aside class="sidebar">
	<div class="level">
		<div class="dropdown" bind:this={dropdown} class:is-active={dropdownActive} on:focusout={handleFocus}>
			<div class="dropdown-trigger">
				<button
					class="button"
					aria-haspopup="true"
					aria-controls="dropdown-menu"
					on:click={() => dropdownActive = !dropdownActive}
				>
					<p class="image is-32x32">
						<img
							class="round"
							src="/128x128.png"
							alt="Home icon"
						/>
					</p>
				</button>
			</div>
			<div class="dropdown-menu" id="dropdown-menu3" role="menu" on:click={() => dropdownActive = false}>
				<div class="dropdown-content">
					<button class="button dropdown-item">
						Settings
					</button>
					<hr class="dropdown-divider" />
					<button class="button dropdown-item" on:click={() => connection.disconnect()}>
						Disconnect
					</button>
				</div>
			</div>
		</div>
		<div class="media-content">
			<p class="control has-icons-right">
				<input class="input" type="text" placeholder="Search" bind:value={searchValue} />
				<span class="icon is-small is-right">
					<i class="mdi mdi-magnify mdi-dark"></i>
				</span>
			</p>
		</div>
	</div>

	<div class="sidebar-content">
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
						<li>User</li>
						<li>User</li>
					</ul>
				</li>
			</ul>
		</div>
	</div>
</aside>

<style lang="scss">
	button {
		background: none;
		border: none;
		border-radius: 0;
	}
	button:focus {
		box-shadow: none;
	}

	.sidebar {
		display: inline-flex;
		flex-direction: column;
		position: absolute;
		top: 0;
		bottom: 0;
		box-sizing: border-box;
		width: var(--channel-tree-width);
		background-color: #eee;
		border-right: rgb(179, 179, 179) 2px solid;
	}

	.sidebar > .level {
		padding: 0.5em;
	}

	.sidebar .level:not(:last-child) {
		margin-bottom: 0;
	}

	.sidebar-content {
		overflow-y: auto;
	}

	.sidebar-content button {
		position: sticky;
		bottom: 0;
		top: 0;
		z-index: 1;
		background-color: #fff;
		box-shadow: 0 0.3em 0.3em #bbb;

		width: 100%;
		justify-content: start;
	}

	button.selectedServerChat {
		background: #ddd;
	}

	.sidebar-content button.chats-header {
		top: 2.2em;
	}

	.sidebar-content > .menu .menu-list li {
		margin: 2em;
	}

	.round {
		border-radius: 100%;
	}

	.dropdown-trigger button {
		margin-right: 0.5em;
		padding: 0;
		height: auto;
	}
</style>
