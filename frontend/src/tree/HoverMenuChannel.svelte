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

<style lang="scss">
</style>
