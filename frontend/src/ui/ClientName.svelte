<script lang="typescript">
	import { GraphQlClient } from "../tree/book";
	import { Message } from "../chat/chat"
	import { getDataColor } from "../util";

	export let client!: GraphQlClient | Message;
	
	let data, name;
	if (client instanceof GraphQlClient) {
		data = client.uid;
		name = client.name;
	} else {
		data = client.invoker?.uid ?? client.invokerName ?? client.displayName;
		name = client.invoker?.name ?? client.displayName;
	}
	let color = getDataColor(data);
</script>

<span style={color}>{name}</span>
