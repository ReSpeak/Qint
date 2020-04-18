<script>
	import Server from "../tree/Server.svelte";

	export let connection;
	let server = connection.book.server;
</script>

<aside class="sidebar">
	<div class="level" style="padding: 0.5em;">
		<div class="dropdown" onclick="dropdown_click">
			<div class="dropdown-trigger">
				<figure class="media-left" style="cursor: pointer;">
					<p class="image is-32x32">
						<img
							class="round"
							src="/128x128.png"
							alt="Home icon"
						/>
					</p>
				</figure>
			</div>
			<div class="dropdown-menu" id="dropdown-menu3" role="menu">
				<div class="dropdown-content">
					<a href="#" class="dropdown-item">{'Options'}</a>
					<hr class="dropdown-divider" />
					<a
						href="#"
						class="dropdown-item"
						onclick="disconnect_click"
					>
						{'Disconnect'}
					</a>
				</div>
			</div>
		</div>
		<div class="media-content">
			<p class="control has-icons-right">
				<input class="input" type="text" placeholder="Search" />
				<span class="icon is-small is-right">
					<i class="mdi mdi-magnify mdi-dark"></i>
				</span>
			</p>
		</div>
	</div>

	<div class="sidebar-content">
		<button class="entry-expand button">
			<span class="expand" class:selected-server="{true}">
				{$server.name}
			</span>
		</button>
		<Server {connection} />

		<button class="entry-expand button chats-header">
			<span class="entry-expand" style="display:flex;">
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
						<li>User</li>
						<li>User</li>
						<li>User</li>
						<li>User</li>
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
	.sidebar {
		display: inline-flex;
		flex-direction: column;
		position: absolute;
		top: 0;
		bottom: 0;
		box-sizing: border-box;
		width: var(--channel-tree-width);
		background-color: #eeeeee;
		border-right: rgb(179, 179, 179) 2px solid;
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

		width: 100%;
		justify-content: start;
		border: none;
	}

	.sidebar-content button.chats-header {
		top: 2em;
	}

	.sidebar-content > .menu .menu-list li {
		margin: 2em;
	}

	.round {
		border-radius: 100%;
	}
</style>
