<script lang="ts">
	import { Connection } from "../connection";
	import MuteButtons from "../bar/MuteButtons.svelte";
	import Icon from "../ui/icon/Icon.svelte";
	import { NodeSelection } from "../app";
	import QuickActionButtons from "../bar/QuickActionButtons.svelte";

	export let connection: Connection;
	$: serverRaw = connection.book.server;
	$: server = $serverRaw;
</script>

<div class="inlineButtons">
	<QuickActionButtons selected={new NodeSelection(connection, server)} />
	<div class="spacer"></div>
	<MuteButtons {connection} />
</div>
<button on:click={() => connection.disconnect()}><Icon name="exit-to-app" />Disconnect</button>
<button><Icon name="comment-plus-outline" />Create channel</button>

<style lang="scss">
	.spacer {
		width: 1em;
	}
</style>
