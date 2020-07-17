<script>
	import BInput from "./ui/BInput.svelte";

	let val = "dummy";
	let inp = "hi";
</script>

<div class="testingList">
	<BInput bind:value={inp} />
	<BInput bind:value={val} />
	<div class="text">{val}</div>

	<input bind:value={inp} />
</div>

<style>
	.testingList {
		border: 1px lightblue solid;
		height: 75vh;
	}

	:global(html, body) {
		background-color: #1a1a1a;
	}

	.text {
		color: wheat;
		white-space: pre;
	}
</style>
