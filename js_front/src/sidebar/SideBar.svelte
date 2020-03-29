<script>
	import Server from "../tree/Server.svelte";

	export let connection;
</script>

<aside class="sidebar">
	<div class="level" style="padding: 0.5em;">
		<div class="dropdown" onclick="dropdown_click">
			<div class="dropdown-trigger">
				<figure class="media-left" style="cursor: pointer;">
					<p class="image is-32x32">
						<img
							class="round"
							src="https://bulma.io/images/placeholders/128x128.png"
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

	<Server {connection} />

	<div class="menu">
		<ul class="menu-list">
			<li>
				<div class="channel-line">
					<a href="#" class="entry-expand">
						<span class="entry-expand" style="display:flex;">
							Splamy (maybe)
						</span>
					</a>
				</div>
				<ul class="menu-list"></ul>
			</li>
		</ul>
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

	.round {
		border-radius: 100%;
	}
</style>
