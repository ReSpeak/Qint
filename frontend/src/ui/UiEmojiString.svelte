<script lang="typescript">
	import type { EmojiData } from "../util";
	import { emojiEncode } from "../util";

	export let data: number[];
	let emoji: EmojiData[];
	$: emoji = emojiEncode(data);
</script>

<div class="emojiString">
	{#each emoji as e}
		<span title={e.description}>{e.emoji}</span>
	{/each}
</div>

<style lang="scss">
</style>
