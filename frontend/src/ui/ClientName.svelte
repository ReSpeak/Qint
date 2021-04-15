<script lang="ts">
	import { ClientBase } from "../bookBase";
	import { Client, GraphQlClient } from "../book";
	import { Message } from "../chat/chat";
	import { getDataColor } from "../util";

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

	$: [color, name] = refreshClient(client);
</script>

<span style="color:{color};">{name}</span>
