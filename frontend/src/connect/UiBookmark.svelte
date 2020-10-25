<script lang="typescript">
	import { BOOKMARK_OFF, BOOKMARK_ON, EDIT_ICON, LONG_DATETIME } from "../util";
	import TsIcon from "../ui/TsIcon.svelte";
	import { Bookmark } from "./bookmark";
	import { ConnectData } from "./connect";
	import { app } from "../app";

	export let bookmark: Bookmark;
	let error: string | undefined = undefined;
	let fullAddress: string;
	$: {
		fullAddress = bookmark.address ?? "";
		if (bookmark.channel !== null) fullAddress += "/" + bookmark.channel.fullPath;
	}

	function doConnect() {
		const channel = bookmark.channel !== null ? Number(bookmark.channel.id) : undefined;
		if (
			bookmark.username !== undefined &&
			bookmark.address !== undefined &&
			bookmark.id !== undefined
		)
			app.connect(
				new ConnectData(
					bookmark.username,
					bookmark.address,
					Number(bookmark.id),
					bookmark.channel?.fullPath,
					channel
				)
			);
	}

	function toggleBookmark() {
		error = undefined;
		bookmark.bookmark = !bookmark.bookmark;
		bookmark.update().catch((err) => {
			console.log("Failed to update bookmark", err);
			error = "Failed to update bookmark";
		});
	}

	function toggleEdit() {
		// TODO
	}
</script>

<div
	class="bookmarkItem"
	on:click={doConnect}
	title={bookmark.server?.name}
	class:bookmark={bookmark.bookmark}>
	<div class="bookmarkIcon">
		<TsIcon
			type="server"
			source={{ icon: bookmark.server?.icon }}
			server={bookmark.server?.hexPublicKey} />
	</div>
	<div class="bookmarkName">{bookmark.name || bookmark.server?.name}</div>
	{#if bookmark.lastUsed}
		<div class="bookmarkInfo" title={bookmark.lastUsed.format(LONG_DATETIME) ?? ''}>{fullAddress}</div>
	{/if}

	<button class="button bookmarkEdit" on:click|stopPropagation={toggleEdit}>
		<i class="mdi mdi-{EDIT_ICON} mdi-24px" />
	</button>
	<button class="button bookmarkStar" on:click|stopPropagation={toggleBookmark} title="Bookmark">
		<i class="mdi mdi-{BOOKMARK_ON} mdi-24px bookmarkOn" />
		<i class="mdi mdi-{BOOKMARK_OFF} mdi-24px bookmarkOff" />
	</button>
	{#if error}<span class="bookmarkError tag is-danger">{error}</span>{/if}
</div>

<style lang="scss">
	.bookmarkItem {
		@extend %unselectable;
		cursor: pointer;
		background-color: change-color(scale-color($background, $lightness: +10%), $alpha: 0.7);
		border-radius: 0.4em;
		margin: 0.5em;
		display: grid;
		justify-content: stretch;
		grid-template-columns: 2em minmax(0, 1fr) 2em;
		width: 100%;
		height: 100%;
	}

	.bookmarkItem:hover {
		background-color: change-color(scale-color($background, $lightness: +15%), $alpha: 0.7);
	}

	.bookmarkItem:hover .bookmarkIcon {
		color: $text-strong;
	}

	.bookmarkIcon {
		grid-column: 1;
		grid-row: 1 / 3;
		color: $text-light;
		display: flex;
		justify-content: center;
		align-items: center;
	}

	.bookmarkName,
	.bookmarkInfo {
		width: 100%;
		height: 100%;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.bookmarkName {
		grid-column: 2;
		grid-row: 1;
	}

	.bookmarkInfo {
		grid-column: 2;
		grid-row: 2;
		color: mix($text, $background, 60%);
		font-size: 0.85em;
	}

	.bookmarkEdit {
		grid-column: 2;
		text-align: center;
		background: none;
		border: none;
		box-shadow: none;
		display: none; // TODO Remove when ready
	}

	.bookmarkStar {
		grid-column: 3;
		grid-row: 1 / 3;
		text-align: center;
		color: $yellow;
		background: none;
		border: none;
		box-shadow: none;
	}

	.bookmarkStar:hover {
		color: scale-color($yellow, $lightness: +60%);
	}

	.bookmarkStar .bookmarkOn {
		display: none;
	}

	.bookmark .bookmarkStar .bookmarkOn {
		display: inherit;
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
