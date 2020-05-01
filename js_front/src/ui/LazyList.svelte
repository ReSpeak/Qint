<script>
	import { sleep } from "../util";
	import { tick } from "svelte";

	let hold_count = 50;
	let fetch_count = 25;
	let pxBeforeLoad = 100; // TODO? could be adjusted dynamically

	// the data elements held by this list
	export let elems;

	// the lowest and highest id currently in the list
	let hold_id_start = 200;
	let hold_id_end = hold_id_start + elems.length;

	// the lowest and highest id that could be retrieved from the source
	let fetch_id_min = 0;
	let fetch_id_max = 999;

	let pan; // The holding list element which has the scrollbar
	let lastScollPos = undefined;

	let fetchTask; // prevent asycn weirness by only allowing one async task

	function* dummies() {
		for (let i = 0; ; i++) {
			yield { id: i, text: "n" + i };
		}
	}

	function handle_scroll(e) {
		// console.log(
		// 	pan.scrollHeight, // complete content
		// 	pan.scrollTop, // current scroll position
		// 	pan.scrollTopMax, // max scoll position
		// 	pan.offsetHeight, // container height
		// 	pan.clientHeight); // inner view height (after subtracting border/padding)
		// ! clientHeight + scrollTopMax == scrollHeight
		let scrollDiff = 0;
		if (lastScollPos !== undefined) {
			scrollDiff = pan.scrollTop - lastScollPos;
			//console.log("diff", scrollDiff);
		}
		lastScollPos = pan.scrollTop;

		if (fetchTask) {
			return;
		}

		const distFromTop = pan.scrollTop;
		const distFromBot = pan.scrollTopMax - pan.scrollTop;

		const wantFetchStart = distFromTop < pxBeforeLoad && scrollDiff <= 0;
		const wantFetchEnd = distFromBot < pxBeforeLoad && scrollDiff >= 0;

		const canLoadBeforeStart = hold_id_start > fetch_id_min;
		const canLoadAfterEnd = hold_id_end < fetch_id_max;

		if (wantFetchStart && canLoadBeforeStart) {
			const count = Math.min(hold_id_start - fetch_id_min, fetch_count);
			const from = hold_id_start - count;
			console.log("want start", from, count);
			fetchTask = load(from, count);
		} else if (wantFetchEnd && canLoadAfterEnd) {
			const from = hold_id_end + 1;
			const count = Math.min(fetch_id_max - hold_id_end, fetch_count);
			console.log("want end", from, count);
			fetchTask = load(from, count);
		}
	}

	async function load(from, count) {
		//await sleep(1000);
		let fElems = dummies()
			.linq()
			.skip(from)
			.take(count)
			.toArray();

		await applyElements(fElems, from);

		fetchTask = undefined;
		//console.log("fetch cleared");
	}

	async function modifyElems(newElems) {
		const lastScrollHeight = pan.scrollHeight;
		elems = newElems;
		await tick();
		var scrollDiff = pan.scrollHeight - lastScrollHeight;
		//console.log("moddiff", scrollDiff);
		pan.scrollTop += scrollDiff;
		lastScollPos += scrollDiff;
	}

	async function applyElements(newElems, from) {
		// TODO:not sure, but I think add + trim could be done in one step
		if (from === hold_id_end + 1) {
			elems = [...elems, ...newElems]; // modification is at the end => safe
			hold_id_end += newElems.length;
			if (elems.length > hold_count) {
				await tick(); // await previous change
				await modifyElems(elems.slice(elems.length - hold_count)); // mofification at start => helper
				hold_id_start = hold_id_end - newElems.length;
			}
		} else if (from + newElems.length == hold_id_start) {
			await modifyElems([...newElems, ...elems]); // mofification at start => helper
			hold_id_start -= newElems.length;
			if (elems.length > hold_count) {
				elems = elems.slice(0, hold_count); // modification is at the end => safe
				hold_id_end = hold_id_start - newElems.length;
			}
		} else {
			// case when jumping to non-adjacent blocks
			// OR: overlapping case, but that should be trimmed to one of the two top one (TODO)
			throw new Error("not impl");
		}
	}
</script>

<div class="lazyList" bind:this="{pan}" on:scroll="{handle_scroll}">
	<div class="scrollPane">
		{#each elems as data}
			<slot {data} />
		{/each}
	</div>
</div>

<style>
	.lazyList {
		border: 1px black solid;
		height: 100px;
		overflow-x: hidden;
		overflow-y: scroll;
	}

	.scrollPane {
	}
</style>
