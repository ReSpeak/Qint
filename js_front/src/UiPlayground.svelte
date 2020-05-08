<script>
	import LazyList from "./ui/LazyList.svelte";
	import { ListFetchDir } from "./ui/lazyList";

	function* dummies(start, count) {
		for (let i = start; i < start + count; i++) {
			yield { id: i, text: "n" + i };
		}
	}

	function fetchElements(idFrom, dir) {
		const min = 0;
		const max = 300;
		const take = 25;

		if (dir === ListFetchDir.Before) {
			const count = Math.min(idFrom.id - min, take);
			const from = Math.max(min, idFrom.id - count);
			return {
				items: dummies(from, count)
					.linq()
					.toArray(),
				hasEnd: from === min,
			};
		} else if (dir === ListFetchDir.Before) {
			const from = idFrom.id + 1;
			const count = Math.min(max - from - 1, take);
			return {
				items: dummies(from, count)
					.linq()
					.toArray(),
				hasEnd: from + count - 1 === max,
			};
		} else {
			return {
				items: dummies(0, 1)
					.linq()
					.toArray(),
				hasEnd: false,
			};
		}
	}

	let minId = { id: 0 };
	let maxId = { id: 0 };
</script>

<span>Before List</span>
<br />
<div class="testingList">
	<LazyList
		{fetchElements}
		compare="{(a, b) => a.id - b.id}"
		bind:fetchIdMin="{minId}"
		bind:fetchIdMax="{maxId}"
		let:data
	>
		<slot>
			<b>{data.id}</b>
			<span style="white-space: pre-wrap;">Elem: {data.text}</span>
			<br />
		</slot>
	</LazyList>
</div>
<span>After List</span>
<button
	on:click="{() => {
		maxId = { id: maxId.id + 1 };
	}}"
>
	Add new Message: {maxId.id}
</button>

<style>
	.testingList {
		border: 1px black solid;
		height: 75vh;
	}
</style>
