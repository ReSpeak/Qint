<script>
	import { afterUpdate } from "svelte";
	import ClientIcon from "../ui/ClientIcon.svelte";
	import { flash } from "../util";

	export let connection;
	export let client;
	let selectedChat = connection.chat.selectedChat;

	let ownClient = client.id === connection.ownClient;
	$: selectedClient = "Client" in $selectedChat && $selectedChat.Client === client.id;
	let div;

	function setChat() {
		connection.chat.selectClient(client);
	}

	afterUpdate(() => {
		flash(div);
	});
</script>

<li>
	<div
		bind:this={div}
		class:ownClient
		class:selectedClient
	>
		<button class="button" on:click={setChat}>
			<ClientIcon {client} {connection} />
			<span>{client.name}</span>
		</button>
	</div>
</li>

<style>
	.button {
		background: none;
		border: none;
		padding: 0.2em 1em 0.2em 1em;
		height: auto;
		width: 100%;
		justify-content: start;
	}
	.button:focus {
		box-shadow: none;
	}

	.ownClient span {
		font-weight: bold;
	}

	.selectedClient {
		background-color: #ddd;
	}
</style>