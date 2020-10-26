<script lang="typescript">
	import { Connection } from "../connection";
	import type { ChannelId } from "../ts";
	import RenderedText from "../ui/RenderedText.svelte";

	export let connection: Connection;
	export let channelId: ChannelId;

	$: channelRaw = connection.book.channels.get(channelId)!;
	$: channel = $channelRaw;
</script>

<div class="name">
	{channel.name}
</div>
{#if channel.topicRendered !== null && channel.topicRendered.length > 0}
<div class="description">
	<RenderedText text={channel.topicRendered} />
</div>
{/if}

<style lang="scss">
</style>
