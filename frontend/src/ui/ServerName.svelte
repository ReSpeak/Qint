<script lang="typescript">
	import { Server } from "../book";
	export let server: Server;
</script>

<span style={server.getColor()}>
	{server.name}
</span>
