<script>
	import LazyList from "./ui/LazyList.svelte";

	function* dummies() {
		for (let i = 0; ; i++) {
			yield { id: i, text: "n" + i };
		}
	}

	let elems = []; //dummies().linq().skip(200).take(10).toArray();
</script>

<span>Before List</span>
<br />
<div class="testingList">
	<LazyList {elems} let:data>
		<slot>
			<b>{data.id}</b>
			<span style="white-space: pre-wrap;">Elem: {data.text}</span>
			<br />
		</slot>
	</LazyList>
</div>
<br />
<span>After List</span>

<style>
	.testingList {
		border: 1px black solid;
		height: 75vh;
	}
</style>
