<script lang="ts">
	import { Channel } from "../book";

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
