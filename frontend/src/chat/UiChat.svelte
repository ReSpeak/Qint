<script lang="typescript">
	// Use scroll-anchoring https://blog.eqrion.net/pin-to-bottom/
	import UiMessage from "./UiMessage.svelte";
	import Icon from "../ui/Icon.svelte";
	import TsIcon from "../ui/TsIcon.svelte";
	import ClientName from "../ui/ClientName.svelte";
	import ServerName from "../ui/ServerName.svelte";
	import UiLazyList from "../ui/UiLazyList.svelte";
	import BInput from "../ui/BInput.svelte";
	import { onDestroy, onMount, tick } from "svelte";
	import { Chat, Message, structuredViewToMd } from "./chat";
	import type { MdFile, MdWithFiles } from "./chat";
	import { ListFetchDir } from "../ui/lazyList";
	import { Connection } from "../connection";
	import { app, NodeSelection } from "../app";
	import { Channel, Client, Server } from "../book";
	import { get, writable } from "svelte/store";
	import type { Readable, Writable } from "svelte/store";
	import { on, SERVER_ICON } from "../util";
	import type { ChatData } from "../bookBase";
	import type { ChannelId } from "../ts";
	import { pathJoin } from "../panel/fileUtil";
	import { TsError } from "../book_events";
	import debug from "debug";
	const log = debug("CHAT"),
		error = debug("error:CHAT");

	export let chat: Chat;

	const selected = app.selectedNode;
	let chatStore = app.transientSettings.chat;

	let developMode = app.transientSettings.ui._developMode;
	let chatList: UiLazyList | undefined;
	let messagesError: unknown | undefined;
	let messageInput: BInput;
	let text = "";
	let command = "";
	let lastDisplayed: Message | undefined;
	let sendError: string | undefined;
	let isSending = false;

	let canChatHere = false;

	let oldSelection: NodeSelection | undefined;
	let connection: Connection | undefined;
	let ownClient: Writable<Client | undefined>;
	$: {
		connection = $selected?.connection;
		ownClient = connection?.book.ownClient ?? writable(undefined);
	}

	let oldOwnChannel: string | undefined;
	let oldCon: string | undefined;
	$: {
		const ownChannel = $ownClient?.channel;
		const con = connection?.backend.id;
		if (ownChannel !== oldOwnChannel || con !== oldCon) {
			oldOwnChannel = ownChannel;
			oldCon = con;
			chatBoxRecheck();
		}
	}

	let sel: NodeSelection | undefined;
	let chatData: Readable<ChatData> | undefined;
	// Note here: we need to check `chatChanged` and `unreadCountChanged` in one
	// update cycle. Otherwise starting the 'wrong' one first will prevent the
	// second one from working correctly.
	// E.g. unreadCountChanged -> async update stared -> chatChanged -> (does nothing)
	$: {
		chatData = $selected?.node.chat;
		if (sel !== $selected) {
			sel = $selected;
			chatChanged();
		} else if (chatData !== undefined && $chatData) {
			unreadCountChanged();
		}
	}

	function chatChanged() {
		if (!chatList || !chatList.clear) return;

		const sel = $selected;
		if (sel === undefined) {
			oldSelection = undefined;
			chatList.clear();
			canChatHere = false;
			return;
		}

		if (!NodeSelection.equals(sel, oldSelection)) {
			if (oldSelection !== undefined) chatStore.save(text, oldSelection);
			text = chatStore.load(sel) ?? "";
			oldSelection = sel;

			chatList.sourceChanged(ListFetchDir.New, ListFetchDir.After);
			chatBoxRecheck();
		}
	}

	function unreadCountChanged() {
		// TODO Also check for sourceChanged, so hot reload with snowpack works
		if (!chatList || !chatList.sourceChanged) return;

		chatList.sourceChanged(ListFetchDir.After, ListFetchDir.After);
	}

	function chatBoxRecheck() {
		const sel = $selected;
		if (sel === undefined) {
			canChatHere = false;
			return;
		}
		const ownChannel = $ownClient?.channel;

		let type = sel.node.qlType;
		if (type === "SERVER" || type === "CLIENT") {
			canChatHere = true;
		} else if (type === "CHANNEL" && sel.node instanceof Channel) {
			canChatHere = ownChannel === undefined || sel.node.id === ownChannel;
		}
		if (canChatHere) {
			(async () => {
				await tick();
				if (messageInput) {
					messageInput.focus();
				}
			})();
		}
	}

	async function sendMessage() {
		try {
			if (isSending) return;
			isSending = true;
			const sel = $selected;
			if (sel === undefined) return;
			let textData = messageInput.getStructuredView();
			if (textData.length === 0) return;
			let channelId: ChannelId = get(sel.connection.book.ownClient)?.channel ?? "";
			let mdData = structuredViewToMd(textData, channelId);
			if (!mdData.text) return;
			sendError = await tryUploadChatImage(sel.connection, mdData, channelId);
			if (sendError === undefined) {
				chat.sendMessage(mdData.text);
				messageInput.clear();
				messageInput.focus();
			}
		} finally {
			isSending = false;
		}
	}

	function chatboxContentChanged() {
		sendError = undefined;
	}

	async function tryUploadChatImage(
		connection: Connection,
		mdData: MdWithFiles,
		channelId: ChannelId
	): Promise<string | undefined> {
		if (mdData.files.length === 0) return undefined;

		function tryUpload(file: MdFile) {
			const [returnCode, promise] = connection.generateReturnCode();
			const uploadDonePromise = connection.backend.fetch(
				`/file${pathJoin(channelId, file.path, file.name)}?return_code=${returnCode}`,
				{
					method: "PUT",
					body: file.blob,
				}
			);
			return promise;
			//return (await request.json()) as ResultDetails;
		}

		async function createFolder(file: MdFile) {
			return await connection.sendChange({
				ChannelCreateDirectory: {
					id: channelId,
					password: "", // TODO
					path: file.path,
				},
			});
		}

		if (mdData.files.length >= 1) {
			let file = mdData.files[0];
			let uplRes = await tryUpload(file);

			if (uplRes !== undefined && uplRes.tsResult !== TsError.Ok) {
				if (uplRes.tsResult === TsError.PermissionsClientInsufficient) {
					log("No permission to upload file");
					return "No permission to upload files to this channel";
				} else if (
					uplRes.tsResult === TsError.FileInvalidPath ||
					uplRes.tsResult === TsError.FileNotFound
				) {
					log("Creating folder for chat images");
					let cresult = await createFolder(file);
					if (cresult !== undefined) {
						log("Could not create folder %o", cresult);
						return "No permission to create folders in this channel";
					}
					uplRes = await tryUpload(file); // XXX log?
				}
				if (uplRes !== undefined && uplRes.tsResult !== TsError.Ok) {
					log("Unknown error %o", uplRes);
					return `Failed to upload images: ${uplRes.tsResult}`;
				}
			}

			if (mdData.files.length > 1) {
				let tasks = mdData.files.slice(1).map((file) => tryUpload(file));
				await Promise.allSettled(tasks);
			}
		}
		return undefined;
	}

	function sendCommand() {
		if (!command || !connection) return;
		connection.sendMessage({
			SendCommand: {
				command,
			},
		});
		command = "";
	}

	async function fetchElements(idFrom: Message | undefined, dir: ListFetchDir) {
		messagesError = undefined;
		try {
			const res = await chat.getMessages(idFrom, dir);
			return res;
		} catch (err) {
			error("Failed to load messages %o", err);
			messagesError = err;
			return Chat.EmptyFetch;
		}
	}

	async function markRead() {
		if (chatData === undefined || lastDisplayed === undefined) return;
		if (document.hasFocus() && lastDisplayed.date > $chatData.lastRead) {
			await chat.setLastRead(lastDisplayed.id, lastDisplayed.date);
		}
	}

	async function viewchanged(ev: CustomEvent<{ first?: Message; last?: Message }>) {
		lastDisplayed = ev.detail.last;
		await markRead();
	}

	onMount(() => {
		chatChanged();
		window.addEventListener("focus", markRead);
	});

	onDestroy(() => {
		chatChanged();
		window.removeEventListener("focus", markRead);
	});
</script>

<div class="chat">
	{#if messagesError}
		<div>
			<article class="message is-danger">
				<div class="message-header">
					<p>Error</p>
				</div>
				<div class="message-body">Failed to fetch messages</div>
			</article>
		</div>
	{:else if $selected !== undefined && sel !== undefined}
		<UiLazyList
			on:viewchanged={viewchanged}
			bind:this={chatList}
			{fetchElements}
			suggestJumpEnd={true}
			notifyViewChanged={chatData !== undefined && $chatData.unreadCount > 0}
			let:item>
			<div slot="loading" class="chatFiller">
				<span>Loading ...</span>
				<Icon name="orbit mdi-spin" />
			</div>
			<div slot="empty" class="chatFiller">Chat history empty ¯\_(ツ)_/¯</div>
			{#if item.displayDateSeparator}
				<div title={item.date.format('L')} class="chat-date">
					<div class="chat-date-line" />
					<span>{item.date.format('LL')}</span>
					<div class="chat-date-line" />
				</div>
			{/if}
			{#if item.displayGroupHeader}
				<div class="invoker-row">
					<div class="invoker-icon chat-left-col">
						{#if item.invoker}
							<TsIcon type="client" source={item.invoker} {connection} />
						{:else}
							<Icon name={SERVER_ICON} />
						{/if}
					</div>
					<div class="invoker-name has-text-weight-bold">
						{#if item.invoker}
							<ClientName client={item.invoker} />
						{:else}Server{/if}
					</div>
				</div>
			{/if}
			<UiMessage
				message={item}
				unread={chatData !== undefined && item.date > $chatData.lastRead}
				nodeSel={sel} />
		</UiLazyList>
		<form class="chat-form" class:hidden={!canChatHere} on:submit|preventDefault={sendMessage}>
			{#if sendError !== undefined}
				<div class="sendError">{sendError}</div>
			{/if}
			<div class="sendCombo">
				<BInput
					bind:this={messageInput}
					value={text}
					on:submit={sendMessage}
					on:structureChanged={chatboxContentChanged}>
					<div slot="placeholder">
						<span>Send to</span>
						<!-- TODO: Remove 'sel !== undefined' when svelte-tool understands it -->
						{#if sel !== undefined && sel.node instanceof Client}
							<ClientName client={sel.node} />
						{:else if sel !== undefined && sel.node instanceof Channel}
							<span> your channel</span>
						{:else if sel !== undefined && sel.node instanceof Server}
							<ServerName connection={sel.connection} />
						{/if}
					</div>
				</BInput>
				<button
					class="button outline-button"
					class:is-loading={isSending}
					name="send"
					type="submit"
					style="height: auto;">Send</button>
			</div>
		</form>
		{#if $developMode}
			<form class="chat-form" on:submit|preventDefault={sendCommand}>
				<div class="sendCombo">
					<BInput bind:value={command} />
					<button class="button" name="send" type="submit" style="height: auto;">Send
						Command</button>
				</div>
			</form>
		{/if}
	{:else}
		<div class="chatFiller">No chat selected</div>
	{/if}
</div>

<style lang="scss">
	@import "./chat_style";

	.chat {
		overflow: hidden;
		display: flex;
		flex-direction: column;
		justify-content: flex-end;
		line-height: 1.2;
		border-right: none;

		// The LazyList
		> :global(.lazyList) {
			flex: 1;
		}
	}

	.chat-form {
		display: flex;
		flex-direction: column;
		height: auto;

		margin: 0.5em;
		margin-top: 0;

		:global(img) {
			max-height: min(50vh, 100%);
			max-width: min(50vw, 100%);
		}
	}

	.sendCombo {
		display: flex;
		box-shadow: 0px -5px 20px -5px rgba(0, 0, 0, 0.3);
		max-height: 50vh;
	}

	.sendError {
		border-radius: 0.25em;
		background-color: $red;
		margin-bottom: 0.5em;
		padding: 0.25em;
	}

	.chat :global(.scrollPane:last-child) {
		padding-bottom: 0.5em;
	}

	@mixin block-margin {
		margin-top: 0.5em;
	}

	.chat-date {
		flex: 1;
		display: flex;
		align-items: center;

		@include block-margin;
		text-align: center;
		color: mix($text, $background, 60%);
	}

	.chat-date-line {
		flex: 1;
		border-top: 1px solid mix($text, $background, 60%);
		margin: {
			left: 0.5em;
			right: 0.5em;
		}
	}

	.invoker-row {
		display: flex;
		align-items: center;
		@include block-margin;
		margin-left: $side-pad-width;
		margin-right: $side-pad-width;
	}

	.chat :global(.chat-left-col) {
		width: 48px;
		padding-right: 0.5em;

		display: flex;
		justify-content: center;
		text-align: center;
	}

	.chatFiller {
		@extend %unselectable;
		width: 100%;
		height: 100%;
		padding: 0 1em 3em 1em;
		display: flex;
		flex-direction: column;
		justify-content: flex-end;
		align-items: center;
		color: gray;
		font-size: xx-large;
		white-space: nowrap;

		:global(.icon) {
			font-size: 72px;
		}
	}
</style>
