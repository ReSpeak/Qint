<script>
	import { afterUpdate } from "svelte";
	import ClientIcon from "../ui/ClientIcon.svelte";
	import { flash } from "../util";

	export let connection;
	export let client;

	let collapsed = false;
	// TODO dummy
	let ownClient = false;
	let selectedClient = false;
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
		class="flex-line"
		class:own-client="{ownClient}"
		class:selected-client="{selectedClient}"
	>
		<button class="button expand" on:click={setChat}>
			<ClientIcon {client} {connection} />
			<span class="expand">{client.name}</span>
		</button>
	</div>
</li>

<style>
	.button {
		background: none;
		border: none;
		padding: 0.2em 1em 0.2em 1em;
		height: auto;
	}
	.button:focus {
		box-shadow: none;
	}
</style>