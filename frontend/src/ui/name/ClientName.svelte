<script lang="ts">
	import { ClientBase } from "../../bookBase";
	import { Client, GraphQlClient } from "../../book";
	import { Message } from "../../chat/uiChat";
	import { getDataColor } from "../../util";
	import { app } from "../app";
	import { Connection } from "../connection";

	export let connection: Connection | undefined = undefined;
	export let client: GraphQlClient | Client | Message;

	let color: string = "";
	let name: string = "";

	function refreshClient(cl: GraphQlClient | Client | Message) {
		let data, name;
		if (cl instanceof ClientBase) {
			data = cl.uid ?? cl.name;
			name = cl.name;
		} else {
			data = cl.invoker?.uid ?? cl.invokerName ?? cl.displayName;
			name = cl.invoker?.name ?? cl.displayName;
		}
		return [getDataColor(data), name];
	}

	function click() {
		if (client instanceof Message) {
			if (client.invoker !== undefined)
				app.select(undefined, client.invoker);
		} else {
			app.select(connection, client);
		}
	}

	$: [color, name] = refreshClient(client);
</script>

<span tabindex="0" class="button noBut" style="color: {color};" on:click={click}>{name}</span>