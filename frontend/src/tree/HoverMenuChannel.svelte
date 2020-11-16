<script lang="typescript">
	import { Channel } from "../book";
	import { Connection } from "../connection";

	export let connection: Connection;
	export let channel: Channel;
</script>

<div class="name">
	{$channel.name}
</div>
{#if $channel.topic !== null && $channel.topic.length > 0}
<div class="description">
	{$channel.topic}
</div>
{/if}

<style lang="scss">
</style>
