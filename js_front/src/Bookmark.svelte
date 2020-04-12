<script>
	import { onMount } from 'svelte';
	import { BOOKMARK_OFF, BOOKMARK_ON, SERVER_ICON } from "./ui/const";

	export let connect;
	export let username;
	export let address;
	export let bookmark;
	let error = undefined;

	function doConnect() {
		connect.username = bookmark.username;
		connect.address = bookmark.address;
		connect.connect();
	}

	function toggle() {
		error = undefined;
		bookmark.bookmark = !bookmark.bookmark;
		bookmark.update().catch((err) => {
			console.log("Failed to update bookmark", err);
			error = "Failed to update bookmark";
		});
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

<div class="bookmarkItem" class:bookmark={bookmark.bookmark} on:mouseover={hover} on:mouseout={leave}>
	<button class="button innerBookmarkItem" on:click={doConnect}>
		<div class="bookmarkIcon"><i class="mdi mdi-{SERVER_ICON} mdi-24px"></i></div>
		<div class="bookmarkName">{bookmark.name || bookmark.server.name}</div>
		<div class="bookmarkInfo" title={bookmark.lastUsed.format()}>Last connected on {bookmark.lastUsed.format("lll")}</div>
	</button>
	<button class="button bookmarkStar" on:click={toggle}>
		<i class="mdi mdi-{BOOKMARK_ON} mdi-24px bookmarkOn"></i>
		<i class="mdi mdi-{BOOKMARK_OFF} mdi-24px bookmarkOff"></i>
	</button>
	{#if error}
		<span class="bookmarkError tag is-danger">{error}</span>
	{/if}
</div>

<style lang="scss">
	.bookmarkItem {
		background-color: #eeea;
		border-radius: 0.4em;
		margin: 0.5em;
		display: grid;
		justify-content: stretch;
		grid-template-columns: auto 2.5em;
		width: 100%;
		height: 100%;
	}

	.innerBookmarkItem {
		padding: 0.2em;
		border: none;
		background: none;
		box-shadow: none;
		display: grid;
		justify-content: stretch;
		grid-template-columns: 2.5em auto;
		width: 100%;
		height: 100%;
	}

	.bookmarkItem:hover {
		background-color: #fffa;
	}

	.bookmarkItem:hover .bookmarkIcon {
		color: #4a4a4a;
	}

	.bookmarkIcon {
		grid-column: 1;
		grid-row: 1 / 3;
		text-align: center;
		color: #777;
	}

	.bookmarkName, .bookmarkInfo {
		justify-self: start;
	}

	.bookmarkName {
		grid-column: 2;
		grid-row: 1;
	}

	.bookmarkInfo {
		grid-column: 2;
		grid-row: 2;
		color: #666;
		font-size: 0.85em;
	}

	.bookmarkStar {
		grid-column: 2;
		height: 100%;
		text-align: center;
		color: #d8b507;
		background: none;
		border: none;
		box-shadow: none;
	}

	.bookmarkStar:hover {
		color: #e8c507;
	}

	.bookmarkStar .bookmarkOn {
		display: none;
	}

	.bookmark .bookmarkStar .bookmarkOn {
		display: inherit;
	}

	.bookmarkOff {
		display: none;
	}

	// Display always on touch screens
	@media (pointer:coarse) {
		.bookmarkOff {
			display: inherit;
		}
	}

	.bookmarkItem:hover .bookmarkStar .bookmarkOff {
		display: inherit;
	}

	.bookmarkItem:hover.bookmark .bookmarkStar .bookmarkOff,
	.bookmark .bookmarkStar .bookmarkOff {
		display: none;
	}

	.bookmarkError {
		grid-row: 2;
		grid-column: 1 / 3;
		justify-self: center;
		margin: 0.2em;
	}
</style>
