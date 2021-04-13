<script lang="typescript">
	import { emojiEncode } from "../util";

	export let data: number[];
	export let maxLength = 10;
	$: emoji = emojiEncode(data).slice(0, maxLength);
</script>

<div class="emojiString">
	{#each emoji as e}
		<span title={e.description}>{e.emoji}</span>
	{/each}
</div>

<style lang="scss">
</style>
