<script lang="typescript">
	import { Channel } from "../book";
	import { Connection } from "../connection";

	export let connection: Connection;
	export let channel: Channel;
</script>

<div class="name">
	{$channel.name}
</div>
{#if $channel.topicRendered !== null && $channel.topicRendered.length > 0}
<div class="description">
	{$channel.topic}
</div>
{/if}

<style lang="scss">
</style>
