<script>
	import { onMount } from 'svelte';
	import { BOOKMARK_OFF, BOOKMARK_ON, SERVER_ICON } from "./ui/const";

	export let connect;
	export let username;
	export let address;
	export let bookmark;

	function doConnect() {
		connect.username = bookmark.username;
		connect.address = bookmark.address;
		connect.connect();
	}

	function hover() {
		username.set(bookmark.username);
		address.set(bookmark.address);
	}

	function leave() {
		username.set(connect.username);
		address.set(connect.address);
	}
</script>

<button class="button bookmark" on:click={doConnect} on:mouseover={hover} on:mouseout={leave}>
	<div class="bookmark-icon"><i class="mdi mdi-{SERVER_ICON} mdi-24px"></i></div>
	<div class="bookmark-name">{bookmark.name || bookmark.server.name}</div>
	<div class="bookmark-info">Last connected on April 1, 2020</div>
	<div class="bookmark-star"><i class="mdi mdi-{BOOKMARK_ON} mdi-24px"></i></div>
</button>

<style lang="scss">
	.bookmark {
		background-color: #eeea;
		border-radius: 0.4em;
		padding: 0.2em;
		margin: 0.5em;
		display: grid;
		justify-content: stretch;
		grid-template-columns: 2.5em auto 2.5em;
		width: 100%;
		height: 100%;
	}

	.bookmark:hover {
		background-color: #fffa;
	}

	.bookmark:hover .bookmark-icon {
		color: #4a4a4a;
	}

	.bookmark-icon {
		grid-column: 1;
		grid-row: 1 / 3;
		text-align: center;
		color: #777;
	}

	.bookmark-name, .bookmark-info {
		justify-self: start;
	}

	.bookmark-name {
		grid-column: 2;
		grid-row: 1;
	}

	.bookmark-info {
		grid-column: 2;
		grid-row: 2;
		color: #666;
		font-size: 0.85em;
	}

	.bookmark-star {
		grid-column: 3;
		grid-row: 1 / 3;
		text-align: center;
		color: #e8c507;
	}
</style>
