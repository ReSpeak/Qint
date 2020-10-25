<script lang="typescript">
	import { Connection } from "../connection";
	import ServerName from "../ui/ServerName.svelte";

	export let connection: Connection;
</script>

<div class="name">
	<ServerName {connection} />
</div>

<style lang="scss">
</style>
