<script>
	import TabList from "./ui/TabList.svelte";
	import TabSlot from "./ui/TabSlot.svelte";
</script>

<div class="testingList">
	<TabList>
		<TabSlot title="Root">HI</TabSlot>
		<TabSlot title="Aside">Yo</TabSlot>
		<TabSlot title="Whaaat">Whaaat</TabSlot>
	</TabList>
</div>

<style>
	.testingList {
		border: 1px lightblue solid;
		height: 75vh;
	}

	:global(html, body) {
		background-color: #1a1a1a;
	}
</style>
