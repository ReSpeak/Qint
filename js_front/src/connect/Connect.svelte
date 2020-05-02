<script>
	import { onMount } from 'svelte';
	import { get, writable } from "svelte/store";
	import { Bookmark } from "./bookmark";
	import self from "./connect";
	import { ConnectionState } from "../connection";
	import Icon from "../ui/Icon.svelte";
	import BookmarkComp from "./Bookmark.svelte";
	import { SERVER_ICON, CLIENT_ICON } from "../util";
	import LoadableVirtualList from "../ui/LoadableVirtualList.svelte";

	export let connection;
	let state = connection.state;
	let error = connection.error;
	let data = new self(connection);
	let username = writable(data.username);
	let address = writable(data.address);
	let bookmarks;
	let bookmarkError;
	let usernameInput;

	function onConnect() {
		if (get(state) === ConnectionState.Disconnected) {
			data.username = get(username);
			data.address = get(address);
			data.connect();
		} else {
			data.reset();
		}
	}

	async function loadBookmarks(fromStart) {
		if (bookmarks.length == 0) {
			// That's not dynamic, but we currently have no pagination
			try {
				return await Bookmark.get();
			} catch (err) {
				console.error("Failed to load bookmarks", err);
				bookmarkError = err;
			}
		}
	}

	onMount(async () => {
		usernameInput.focus();
		let recent = await Bookmark.getRecent();
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
	{#if $error}
		<article class="connect-error message is-danger">
			<div class="message-header">
				<p>Error</p>
				<button class="delete" aria-label="delete" on:click={() => error.set(undefined)}></button>
			</div>
			<div class="message-body">
				{$error}
			</div>
		</article>
	{/if}
	<div class="inner-connect-container">
		<div class="connect-blur"></div>
		<form class="connect-form" on:submit|preventDefault={onConnect}>
			<div>
				<p class="control has-icons-left">
					<input
						bind:this={usernameInput}
						bind:value={$username}
						name="username"
						id="username"
						class="input"
						type="text"
						placeholder="Username"
						disabled={$state !== ConnectionState.Disconnected}
					/>
					<Icon name={CLIENT_ICON} is_left />
				</p>
			</div>
			<div>
				<p class="control has-icons-left">
					<input
						bind:value={$address}
						name="server"
						id="server"
						class="input"
						type="text"
						placeholder="Server"
						disabled="{$state !== ConnectionState.Disconnected}"
					/>
					<Icon name={SERVER_ICON} is_left />
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
		<div class="bookmark-blur"></div>
		<div class="bookmark-list">
			{#if bookmarkError}
				<div>
					<article class="message is-danger">
						<div class="message-header">
							<p>Error</p>
						</div>
						<div class="message-body">
							Failed to fetch bookmarks, is Qint running?
						</div>
					</article>
				</div>
			{:else}
				<LoadableVirtualList bind:items={bookmarks} loadMore={loadBookmarks} let:item>
					<div slot="loading" class="loader"></div>
					<BookmarkComp connect={data} {username} {address} bookmark={item}/>
				</LoadableVirtualList>
			{/if}
		</div>
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

	.bookmark-list :global(svelte-virtual-list), .bookmark-list > div {
		border-radius: 0 0 0.4em 0.4em;
		box-shadow: 0 0.3em 0.3em #0005;
	}

	// TODO the error message is broken (start without proxy running)
	.bookmark-list :global(svelte-virtual-list-viewport), .bookmark-list .message {
		background-color: #eefa;
		min-height: 30vh;
		max-height: 50vh;
		padding: 0.5em;
	}

	@media (min-width: 35em) {
		.bookmark-list :global(svelte-virtual-list-viewport), .bookmark-list .message {
			padding: 1em 8em 4em 8em;
		}
	}
</style>
