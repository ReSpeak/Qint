<script lang="typescript">
	import { onMount } from "svelte";
	import { Bookmark } from "./bookmark";
	import { ConnectData } from "./connect";
	import Icon from "../ui/Icon.svelte";
	import UiBookmark from "./UiBookmark.svelte";
	import { Book, Channel } from "../book";
	import UiChannel from "../tree/UiChannel.svelte";
	import type { ChannelId, Uid } from "../ts";
	import { SERVER_ICON, CLIENT_ICON, hexEncode } from "../util";
	import { app } from "../app";
	import { backend } from "../backend/backend";

	export let data: ConnectData;
	let addressInput: HTMLInputElement;
	// The channel part of the address, if empty, the channel popup will be hidden
	let channelPart = "";
	// The address used to load the channels.
	let channelsAddress = "";
	let server: string | undefined = undefined;
	// The channels directly under the server, sub-channels are stored as children.
	let channels: Channel[] = [];
	let address: string = "";

	$: {
		address =
			data.address +
			(data.channel !== undefined ? "/" + data.channel
			: data.channelId !== undefined ? "//" + data.channelId
			: "");
		if (addressInput !== undefined) {
			changeChannels();
		}
	}

	function onConnect() {
		app.connect(data.clone());
	}

	function onNameChange() {
		data.bookmark = undefined;
	}

	async function onAddressChange() {
		data.bookmark = undefined;
		data.channelId = undefined;
		await changeChannels();
	}

	async function changeChannels() {
		const sep = address.indexOf("/");
		if (sep !== -1) {
			data.address = address.slice(0, sep);
			data.channel = address.slice(sep + 1);
		} else {
			data.address = address;
			data.channel = undefined;
		}

		if (
			sep !== -1 &&
			addressInput.selectionStart !== null &&
			addressInput.selectionStart >= sep
		) {
			// Show channel popup
			const addr = address.substr(0, sep);
			if (addr !== channelsAddress) {
				channels = await loadChannels(address.substr(0, sep));
				channelsAddress = addr;
			}
			if (channelPart !== address.substr(sep + 1)) channelPart = address.substr(sep + 1);
		} else {
			if (channelPart !== "") channelPart = "";
		}
	}

	async function loadChannels(address: string): Promise<Channel[]> {
		try {
			const query = await backend.graphql<{serverByAddress:{uid: Uid, channels:Channel[]}}>(
				`
					query GetChannels($address: String!) {
						serverByAddress(address: $address) {
							uid
							channels(includeDeleted: false) {
								id
								parent
								order
								name
								icon
							}
						}
					}
				`,
				{
					address,
				}
			);
			if (query.data.serverByAddress !== null) {
				server = hexEncode(query.data.serverByAddress.uid);
				let channels: Map<ChannelId, Channel> = new Map(
					query.data.serverByAddress.channels.map((c: any) => {
						let channel = Channel.fromGraphql(c);
						return [channel.id, channel];
					})
				);
				let topChannels: Channel[] = [];
				// Get into tree form
				for (let c of channels.values()) {
					// Add to parent
					if (c.parent !== null) {
						let children = channels.get(c.parent)!.channels;
						children.update((cs) => {
							Book.addChannelSorted(cs, c);
							return cs;
						});
					} else {
						Book.addChannelSorted(topChannels, c);
					}
				}
				return topChannels;
			}
			return [];
		} catch (err) {
			console.error("Failed to load channels", err);
			throw err;
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
		addressInput.focus();
		const recent = await Bookmark.getRecent();
		if (recent) {
			if (data.name === "")
				data.name = recent.username ?? "";
			if (address === "") {
				data.address = recent.address ?? "";
				if (recent.channel !== null) {
					data.channel = recent.channel.fullPath;
					data.channelId = recent.channel.id;
				}
				data.bookmark = recent.id;
			}
		}
	});
</script>

<div class="connect-container">
	<div class="inner-connect-container">
		<div class="connect-blur blur" />
		<form class="connect-form blur-shade" on:submit|preventDefault={onConnect}>
			<div>
				<p class="control has-icons-left">
					<input
						bind:value={data.name}
						on:input={onNameChange}
						name="username"
						id="username"
						class="input"
						type="text"
						title="Username"
						placeholder="Username" />
					<Icon name={CLIENT_ICON} isLeft />
				</p>
			</div>
			<div>
				<p class="control has-icons-left">
					<input
						bind:this={addressInput}
						bind:value={address}
						on:input={onAddressChange}
						name="server"
						id="server"
						class="input"
						type="text"
						title="Server address"
						placeholder="Server" />
					<Icon name={SERVER_ICON} isLeft />
				</p>
			</div>
			<div>
				<button class="button is-primary" name="connect" type="submit">
					Connect
				</button>
			</div>
		</form>
		{#if channelPart !== ''}
			<div class="menu channel-list">
				<ul class="menu-list">
					{#each channels as channel (channel.id)}
						<UiChannel
							{server}
							filter={channelPart}
							filterStartFromRoot={true}
							{channel} />
					{/each}
				</ul>
			</div>
		{/if}
	</div>

	<div class="bookmark-container">
		<div class="bookmark-blur blur" />
		<div class="bookmark-list blur-shade">
			{#await loadBookmarks()}
				<div>Loading…</div>
			{:then bookmarks}
				<div class="viewContainer">
					<div class="scollPane">
						{#each bookmarks as item}
							<UiBookmark bookmark={item} />
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

	// .connect-form > div button .loader {
	// 	margin-right: 1.5em;
	// }

	// .connect-error {
	// 	max-width: 100%;
	// 	width: 40em;

	// 	position: relative;
	// 	top: 5%;
	// 	margin: auto auto;
	// }

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
