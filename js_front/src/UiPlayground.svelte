<script>
	import LazyList from "./ui/LazyList.svelte";
	import { ListFetchDir } from "./ui/lazyList";
	import { sleep } from "./util";

	function* dummies(start, count) {
		for (let i = start; i < start + count; i++) {
			yield { id: i, text: "n" + i };
		}
	}

	async function fetchElements(idFrom, dir) {
		await sleep(100);
		const min = minId;
		const max = maxId;
		const take = 25;

		if (dir === ListFetchDir.Before) {
			const count = Math.min(idFrom.id - min, take);
			const from = Math.max(min, idFrom.id - count);
			return {
				items: dummies(from, count)
					.linq()
					.toArray(),
				canLoadBeforeStart: from > min,
				canLoadAfterEnd: from + count < max,
			};
		} else if (dir === ListFetchDir.After) {
			const from = idFrom.id + 1;
			const count = Math.min(max - from, take);
			return {
				items: dummies(from, count)
					.linq()
					.toArray(),
				canLoadBeforeStart: idFrom.id > min,
				canLoadAfterEnd: from + count < max,
			};
		} else {
			return {
				items: dummies(0, 1)
					.linq()
					.toArray(),
				canLoadBeforeStart: 0 > min,
				canLoadAfterEnd: 1 < max,
			};
		}
	}

	let minId = 0;
	let maxId = 5;
	let myLazyList;
</script>

<span>Before List</span>
<br />
<div class="testingList">
	<LazyList
		{fetchElements}
		let:item
		bind:this={myLazyList}
	>
		<slot>
			<b>{item.id}</b>
			<span style="white-space: pre-wrap;">Elem: {item.text}</span>
			<br />
		</slot>
	</LazyList>
</div>
<span>After List</span>
<button
	on:click="{() => {
		maxId += 10;
		myLazyList.sourceChanged(ListFetchDir.After);
	}}"
>
	Add new Message: {maxId}
</button>

<style>
	.testingList {
		border: 1px black solid;
		height: 75vh;
	}
</style>
