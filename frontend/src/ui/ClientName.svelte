<script lang="typescript">
	import { GraphQlClient } from "../book";
	import { Message } from "../chat/chat";
	import { getDataColor } from "../util";

	export let client!: GraphQlClient | Message;

	let color: string = "";
	let name: string = "";

	function refreshClient(cl: GraphQlClient | Message) {
		let data, name;
		if (cl instanceof GraphQlClient) {
			data = cl.uid;
			name = cl.name;
		} else {
			data = cl.invoker?.uid ?? cl.invokerName ?? cl.displayName;
			name = cl.invoker?.name ?? cl.displayName;
		}
		return [getDataColor(data), name];
	}

	$: [color, name] = refreshClient(client);
</script>

<span style={color}>{name}</span>
