<script lang="typescript">
	import { TsErrorDescription } from "../book_events";
	import type { ResultDetails } from "../backend/ws";

	export let result: ResultDetails;
</script>

<div class="changeResult">
	{#if result.tsResult !== undefined}
		{TsErrorDescription(result.tsResult)}
	{:else}
		{result.description}
	{/if}
</div>

<style lang="scss">
</style>
