<script lang="typescript">
	import { Connection } from "../connection";
	import type { ChannelId } from "../ts";

	export let connection: Connection;
	export let channelId: ChannelId;

	$: channelRaw = connection.book.channels.get(channelId)!;
	$: channel = $channelRaw;
</script>

<div class="name">
	{channel.name}
</div>
{#if channel.topic !== null && channel.topic.length > 0}
<div class="topic">
	{channel.topic}
</div>
{/if}

<style lang="scss">
	.topic {
		font-size: 0.85em;
	}
</style>
