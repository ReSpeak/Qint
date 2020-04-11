<script>
	import { onMount } from 'svelte';
	import { get, writable } from "svelte/store";
	import { getRecent, Bookmark } from "./bookmark";
	import self from "./connect";
	import Icon from "./ui/Icon.svelte";
	import BookmarkComp from "./Bookmark.svelte";
	import { BOOKMARK_OFF, BOOKMARK_ON, SERVER_ICON, CLIENT_ICON } from "./ui/const";

	export let connection;
	let data = new self(connection);
	let username = writable(data.username);
	let address = writable(data.address);
	let bookmarks = Bookmark.get();

	onMount(async () => {
		let recent = await getRecent();
		if (recent.data.mostRecentBookmark) {
			if (data.username === "") {
				data.username = recent.data.mostRecentBookmark.username;
				username.set(data.username);
			}
			if (data.address === "") {
				data.address = recent.data.mostRecentBookmark.address;
				address.set(data.address);
			}
		}
	});
</script>

<div class="connect-container">
	<div class="inner-connect-container">
		<div class="connect-blur"></div>
		<form class="connect-form" on:submit|preventDefault="{() => data.connect()}">
			<div>
				<p class="control has-icons-left">
					<input
						bind:value="{$username}"
						name="username"
						id="username"
						class="input"
						type="text"
						placeholder="Username"
					/>
					<Icon name="{CLIENT_ICON}" is_left />
				</p>
			</div>
			<div>
				<p class="control has-icons-left">
					<input
						bind:value="{$address}"
						name="server"
						id="server"
						class="input"
						type="text"
						placeholder="Server"
					/>
					<Icon name="{SERVER_ICON}" is_left />
				</p>
			</div>
			<div>
				<button class="button is-primary" name="connect" type="submit">
					Connect
				</button>
			</div>
		</form>
	</div>

	<div class="bookmark-container">
		<div class="bookmark-blur"></div>
		<ul class="bookmark-list">
			{#await bookmarks}
			<li><i>spinner</i> Loading bookmarks</li>
			{:then list}
			{#each list as bookmark, i}
			<li>
				<BookmarkComp connect={data} {username} {address} {bookmark}/>
			</li>
			{/each}
			{:catch error}
			<li>Failed to fetch bookmarks, is Qint running?</li>
			{/await}
		</ul>
	</div>
</div>

<style lang="scss">
	.connect-container {
		/* https://www.goodfreephotos.com/other-landscapes/scenic-view-of-the-mountains-and-pond-landscape.jpg.php */
		background: url("/background.jpg") repeat fixed center center / cover;
		position: absolute;
		top: 0;
		bottom: 0;
		left: 0;
		right: 0;
		overflow: auto;
	}

	.inner-connect-container {
		max-width: 100%;
		width: 40em;

		position: relative;
		top: 10%;
		margin: auto auto;
	}

	.connect-blur::before {
		filter: blur(5px);
		background: url("/background.jpg") repeat fixed center center / cover;
		content: "";
		position: absolute;
		top: 0;
		left: 0;
		width: 100%;
		height: 100%;
	}

	.connect-blur {
		position: absolute;
		top: 0;
		bottom: 0;
		left: 0;
		right: 0;
		background-color: #dffc;
		border-radius: 0.4em 0.4em 0 0;
		box-shadow: 0 0.3em 0.3em #0005;
	}

	.connect-form {
		position: relative;
		background-color: #eefa;
		border-radius: 0.4em 0.4em 0 0;
		box-shadow: 0 0.3em 0.3em #0005;
		padding: 0.5em;
	}

	@media (min-width: 35em) {
		.connect-form {
			padding: 4em 8em 4em 8em;
		}
	}

	.connect-form > div {
		box-sizing: border-box;
		left: 0;
		right: 0;
		margin: 1em;
	}

	.connect-form > div input:not([type="checkbox"]),
	.connect-form > div button {
		box-sizing: border-box;
		width: 100%;
	}


	.bookmark-container {
		max-width: 100%;
		width: 40em;

		position: relative;
		top: calc(10% + 0.4em);
		margin-left: auto;
		margin-right: auto;
		margin-bottom: 5em;
	}

	.bookmark-blur::before {
		filter: blur(5px);
		background: url("/background.jpg") repeat fixed center center / cover;
		content: "";
		position: absolute;
		top: 0;
		left: 0;
		width: 100%;
		height: 100%;
	}

	.bookmark-blur {
		position: absolute;
		top: 0;
		bottom: 0;
		left: 0;
		right: 0;
		background-color: #dffc;
		border-radius: 0 0 0.4em 0.4em;
		box-shadow: 0 0.3em 0.3em #0005;
	}

	.bookmark-list {
		position: relative;
		height: 100%;
		width: 100%;
		min-height: 30vh;
		max-height: 50vh;
		background-color: #eefa;
		border-radius: 0 0 0.4em 0.4em;
		box-shadow: 0 0.3em 0.3em #0005;
		padding: 0.5em;
		overflow-y: auto;
	}

	@media (min-width: 35em) {
		.bookmark-list {
			padding: 1em 8em 4em 8em;
		}
	}

	.bookmark {
		background-color: #eeea;
		border-radius: 0.4em;
		padding: 0.2em;
		margin: 0.5em;
		display: grid;
		justify-content: stretch;
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
		color: #666;
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
