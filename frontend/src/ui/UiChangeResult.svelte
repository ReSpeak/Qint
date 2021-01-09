<script lang="typescript">
	import type { ResultDetails } from "../backend/ws";

	export let result: ResultDetails;
</script>

<div class="changeResult">
	{result.tsResult ?? result.description}
</div>

<style lang="scss">
</style>
