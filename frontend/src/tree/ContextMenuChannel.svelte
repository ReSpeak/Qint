<script lang="ts">
	import { Channel } from "../book";
	import Icon from "../ui/icon/Icon.svelte";
	import { NodeSelection } from "../app";
	import { Connection } from "../connection";
	import QuickActionButtons from "../bar/QuickActionButtons.svelte";
	import { createEventDispatcher } from "svelte";

	const dispatch = createEventDispatcher<{
		switchChannel: undefined;
	}>();

	export let connection: Connection;
	export let channel: Channel;
</script>

<div class="inlineButtons">
	<QuickActionButtons selected={new NodeSelection(connection, channel)} />
</div>
<button on:click={() => dispatch("switchChannel")}><Icon name="shoe-print" />Join</button>
<button><Icon name="close" />Remove</button>
<button><Icon name="comment-plus-outline" />Create subchannel</button>

<style lang="scss">
</style>
