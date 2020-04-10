<script>
	import Message from "./Message.svelte";
	import Icon from "../ui/Icon.svelte";
	import { DateSeparator } from "./chat";
	// import { sleep } from "../util";

	// let load_morrr;

	export let connection;
	let chat = connection.chat;
	let messages = chat.groupedMessages;
	let composingCommand = "";

	let loadMsgTask = Promise.resolve();
	// console.log(connection);
	// let msgs = connection.messages;
	// let selected_chat = connection.selected_chat;

	// connection.selected_chat.subscribe(_ => handleClick());

	// async function request() {
	// 	//const resp = await fetch(`/messages/${}`);
	// 	//const data = await resp.json();
	// 	await sleep(500);
	// 	console.log("before");
	// 	let data = [{ message: "hallo", user: $selected_chat }];
	// 	msgs.update(m => [...m, ...data]);
	// 	console.log("after");
	// }

	// //waiting_for_msg = request();
	// function handleClick() {
	// 	load_morrr = request();
	// }

	// function changeChat() {
	// 	selected_chat.update(s => "user" + Math.random());
	// }

	function sendMessage(e) {
		e.preventDefault();
		chat.sendMessage();
		chat.composing = "";
	}

	function sendCommand(e) {
		e.preventDefault();
		connection.sendRawMessage(composingCommand);
		composingCommand = "";
	}
</script>

<div class="chat">
	<ul class="chat-messages">
		{#await loadMsgTask}
			<div
				class="is-loading"
				style="color: gray; font-style: italic; text-align: center;"
			>
				Loading…
			</div>
		{:then}
			{#each $messages as group}
				{#if group instanceof DateSeparator}
					<div title="{group.date.format('L')}" class="chat-date">
						{group.date.format('LL')}
					</div>
				{:else}
					<div class="invoker-icon">
						<Icon name="account" />
					</div>
					<div
						class="invoker-name has-text-weight-bold"
						style="user_color"
					>
						{group.user}
					</div>

					{#each group.messages as message}
						<Message {message} />
					{/each}
				{/if}
			{/each}
			<div class="chat-end"></div>
		{:catch}
			<div style="color: red; font-style: italic; text-align: center;">
				Failed to load.
			</div>
		{/await}
	</ul>
	<form class="chat-form" on:submit="{sendMessage}">
		<textarea bind:value="{chat.composing}" class="input auto_height" name="message"></textarea>
		<button class="button" name="send" type="submit">Send</button>
	</form>
	<form class="chat-form" on:submit="{sendCommand}">
		<textarea bind:value="{composingCommand}" class="input auto_height" name="message" type="text"></textarea>
		<button class="button" name="send" type="submit">Send Command</button>
	</form>
</div>

<style lang="scss">
	.chat {
		display: inline-flex;
		flex-direction: column;
		justify-content: flex-end;
		position: absolute;
		top: 0;
		bottom: 0;
		left: var(--channel-tree-width);
		right: 0;
	}

	.chat-date {
		grid-column-start: 1;
		grid-column-end: 3;
		border-top: 1px solid gray;
		margin: 1em 1em 0em 1em;
		text-align: center;
		color: gray;
	}

	.chat-form {
		display: flex;
		width: 100%;
		padding: 0.5em;

		box-shadow: 0px -5px 20px -5px #bbb;
	}

	.chat-form > * {
		box-sizing: border-box;
		height: 2em;
		font-size: 1em;
	}

	.chat-form input {
		flex-grow: 1;
		/*background-color: white;
	color: black;
	border-top: 1px solid black;
	border-right: 1px solid black;
	border-left: none;
	border-bottom: none;*/
	}

	/*.chat-form button {
	background-color: white;
	color: blue;
	vertical-align: middle;
	border-top: 1px solid black;
	border-left: none;
	border-right: none;
	border-bottom: none;
	}*/

	.chat-messages {
		padding: 0.5em;
		overflow-y: auto;
		display: grid;
		grid-template-columns: min-content minmax(0, 1fr);
	}

	.invoker-icon,
	.invoker-name,
	.message-time,
	.message-content {
		padding: 0.1em 0.5em;
	}

	.invoker-icon {
		grid-column: 1;
	}

	.invoker-name {
		grid-column: 2;
		margin-top: auto;
		margin-bottom: auto;
		//font-size: 0.8em;
	}
</style>
