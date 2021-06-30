<script lang="ts">
	import { escapeHtml, ignoreCaseRegex } from "../../util";
	export let filter: string;
	export let content: string;

	$: filteredContent = applyFilter(filter, content);

	function applyFilter(filter: string, content: string): string {
		if (filter === "") {
			return escapeHtml(content);
		} else {
			return escapeHtml(content).replace(
				ignoreCaseRegex(escapeHtml(filter)),
				'<span class="filterHighlight"><span>$&</span></span>'
			);
		}
	}
</script>

<span>
	{@html filteredContent}
</span>

<style lang="scss">
</style>
