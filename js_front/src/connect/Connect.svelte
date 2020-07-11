<script lang="typescript">
	import { onMount } from "svelte";
	import { get, writable } from "svelte/store";
	import { Bookmark } from "./bookmark";
	import self from "./connect";
	import { ConnectionState, Connection } from "../connection";
	import Icon from "../ui/Icon.svelte";
	import BookmarkComp from "./Bookmark.svelte";
	import { SERVER_ICON, CLIENT_ICON } from "../util";
	import LoadableVirtualList from "../ui/LoadableVirtualList.svelte";

	export let connection!: Connection;
	let state = connection.state;
	let error = connection.error;
	let data = new self(connection);
	let username = writable(data.username);
	let address = writable(data.address);
	let usernameInput!: HTMLInputElement;

	function onConnect() {
		if (get(state) === ConnectionState.Disconnected) {
			data.username = get(username);
			data.address = get(address);
			data.connect();
		} else {
			data.reset();
		}
	}

	async function loadBookmarks() {
		// That's not dynamic, but we currently have no pagination
		try {
			return await Bookmark.get();
		} catch (err) {
			console.error("Failed to load bookmarks", err);
			throw err;
		}
	}

	onMount(async () => {
		usernameInput.focus();
		let recent = await Bookmark.getRecent();
		if (recent) {
			if (data.username === "") {
				data.username = recent.username ?? "";
				username.set(data.username);
			}
			if (data.address === "") {
				data.address = recent.address ?? "";
				address.set(data.address);
			}
		}
	});
</script>

<div class="connect-container">
	{#if $error}
		<article class="connect-error message is-danger">
			<div class="message-header">
				<p>Error</p>
				<button
					class="delete"
					aria-label="delete"
					on:click="{() => error.set(undefined)}"
				></button>
			</div>
			<div class="message-body">{$error}</div>
		</article>
	{/if}
	<div class="inner-connect-container">
		<div class="connect-blur blur"></div>
		<form class="connect-form blur-shade" on:submit|preventDefault="{onConnect}">
			<div>
				<p class="control has-icons-left">
					<input
						bind:this="{usernameInput}"
						bind:value="{$username}"
						name="username"
						id="username"
						class="input"
						type="text"
						placeholder="Username"
						disabled="{$state !== ConnectionState.Disconnected}"
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
						disabled="{$state !== ConnectionState.Disconnected}"
					/>
					<Icon name="{SERVER_ICON}" is_left />
				</p>
			</div>
			<div>
				<button class="button is-primary" name="connect" type="submit">
					{#if $state === ConnectionState.Disconnected}
						Connect
					{:else}
						<div class="loader"></div>
						Cancel
					{/if}
				</button>
			</div>
		</form>
	</div>

	<div class="bookmark-container">
		<div class="bookmark-blur blur"></div>
		<div class="bookmark-list blur-shade">
			{#await loadBookmarks()}
				<div>Loading…</div>
			{:then bookmarks}
				<div class="viewContainer">
					<div class="scollPane">
						{#each bookmarks as item}
							<BookmarkComp connect="{data}" {username} {address} bookmark="{item}" />
						{/each}
					</div>
				</div>
			{:catch bookmarkError}
				<article class="message is-danger">
					<div class="message-header">
						<p>Error</p>
					</div>
					<div class="message-body">
						<span>Failed to fetch bookmarks, is Qint running?</span>
						<br />
						<span>Reason: {bookmarkError.message}</span>
					</div>
				</article>
			{/await}
		</div>
	</div>
</div>

<style lang="scss">
	/* background.jpg: https://www.goodfreephotos.com/other-landscapes/scenic-view-of-the-mountains-and-pond-landscape.jpg.php */
	/* background-dark.jpg: https://www.goodfreephotos.com/canada/alberta/jasper-national-park/night-landscape-reflection-and-aurora-in-jasper-national-park-alberta-canada.jpg.php */
	$background-image: "/background-dark.jpg";

	.connect-container {
		background: url($background-image) repeat fixed center center / cover;
		position: absolute;
		top: 0;
		bottom: 0;
		left: 0;
		right: 0;
		overflow: auto;

		> div {
			box-shadow: 0 0.3em 0.3em rgba(0, 0, 0, 0.5);
		}
	}

	.blur {
		position: absolute;
		top: 0;
		bottom: 0;
		left: 0;
		right: 0;
		background-color: change-color($background, $alpha: 0.8);

		&::before {
			filter: blur(5px);
			background: url($background-image) repeat fixed center center / cover;
			content: "";
			position: absolute;
			top: 0;
			left: 0;
			width: 100%;
			height: 100%;
		}
	}

	.blur-shade {
		position: relative;
		background-color: change-color($background, $alpha: 0.7);
	}

	.connect-blur,
	.connect-form {
		border-radius: 0.4em 0.4em 0 0;
	}

	.bookmark-blur,
	.bookmark-list {
		border-radius: 0 0 0.4em 0.4em;
	}

	.inner-connect-container {
		max-width: 100%;
		width: 40em;

		position: relative;
		top: 10%;
		margin: auto auto;
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

	.connect-form > div button .loader {
		margin-right: 1.5em;
	}

	.connect-error {
		max-width: 100%;
		width: 40em;

		position: relative;
		top: 5%;
		margin: auto auto;
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

	.bookmark-list .scollPane {
		min-height: 30vh;
		max-height: 50vh;
	}

	.bookmark-list .message {
		background-color: rgba(0, 0, 0, 0);
	}
	.message-body {
		position: relative;
		background-color: #fff;
	}

	@media (min-width: 35em) {
		.bookmark-list .scollPane,
		.bookmark-list .message {
			padding: 1em 8em 4em 8em;
		}
	}

	// Temporary list hacks until LazyList is used for bookmarks

	.viewContainer {
		display: block;
		position: relative;
		overflow-y: hidden;
	}

	.scollPane {
		position: relative;
		overflow-y: auto;
	}
</style>
