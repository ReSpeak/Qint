<script>
	import { escapeHtml, ignoreCaseRegex } from "../util";
	export let filter;
	export let content;

	$: filteredContent = applyFilter(filter, content);

	function applyFilter(filter, content) {
		if (filter === "") {
			return escapeHtml(content);
		} else {
			return escapeHtml(content).replace(ignoreCaseRegex(escapeHtml(filter)), '<span class="filterHighlight"><span>$&</span></span>');
		}
	}
</script>

<span>{@html filteredContent}</span>

<style lang="scss">
	span > :global(.filterHighlight) {
		position: relative;
	}

	span > :global(.filterHighlight > span) {
		position: relative;
		z-index: 1;
	}

	span > :global(.filterHighlight):after {
		content: '';
		display: block;
		position: absolute;
		top: 0;
		bottom: 0;
		left: 0;
		right: 0;

		background-color: change-color(mix($warning, $background, 80%), $alpha: 0.8);
		border: 1px solid #bb0;
		border-radius: 0.2em;
	}
</style>