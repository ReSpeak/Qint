<script lang="ts">
	import { Channel } from "../book";
	import Icon from "../ui/icon/Icon.svelte";
	import { NodeSelection } from "../app";
	import { Connection } from "../connection";
	import QuickActionButtons from "../bar/QuickActionButtons.svelte";

	export let connection: Connection;
	export let channel: Channel;
</script>

<div class="inlineButtons">
	<QuickActionButtons selected={new NodeSelection(connection, channel)} />
</div>
<button><Icon name="close" />Remove</button>
<button><Icon name="comment-plus-outline" />Create subchannel</button>

<style lang="scss">
</style>
