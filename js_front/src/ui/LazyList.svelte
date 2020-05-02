<script>
	import { sleep, assert } from "../util";
	import { tick, onMount } from "svelte";

	// the data elements held by this list
	export let elems;

	let fetchCount = 25;
	let pxBeforeLoad = 100; // TODO? could be adjusted dynamically
	let pxBufferBothSides = pxBeforeLoad * 2;
	// The amout of items that will be removed when out of view
	let nItemsToRemove = 25;
	// How far the nThItemsToRemove has to be out of view to remove the batch
	let nThItemDistanceToView = 200;

	// Require the minimum distance before deleting an item to be higher
	// than the minimun size the list wants to buffer.
	// Otherwise we might end in an loop of adding and removing a side.
	assert(nThItemDistanceToView > pxBeforeLoad);

	// the lowest and highest id currently in the list
	let holdIdStart = 200;
	let holdIdEnd = holdIdStart + elems.length;

	// the lowest and highest id that could be retrieved from the source
	let fetchIdMin = 0;
	let fetchIdMax = 999;

	// The holding list element which has the scrollbar
	let pan;
	// Utility holder to calculate `scrollDiff`
	let lastScollPos = undefined;
	// In which direction and how far the content has scrolled since last check
	// >0 down, <0 up
	let scrollDiff = 0;

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
		// 	pan.clientHeight // inner view height (after subtracting border/padding)
		// 	ELEM.offsetTop, // is the bottom of the element measured from the top of the container
		// );
		// ! clientHeight + scrollTopMax == scrollHeight
		if (lastScollPos !== undefined) {
			scrollDiff = pan.scrollTop - lastScollPos;
			//console.log("diff", scrollDiff);
		}
		lastScollPos = pan.scrollTop;

		if (fetchTask) {
			return;
		}

		start_fill();
	}

	function start_fill() {
		fetchTask = fill();
	}

	async function fill() {
		for (let i = 0; i < 10; i++) {
			if (i == 9) throw Error("yah, thats a loop");
			if (await fill_loop()) break;
		}
		console.log("Task done");
		fetchTask = undefined;
	}

	/**
	 * Returns true when the list is satisfied with loading data
	 */
	async function fill_loop() {
		await tryTrimEnd();
		const distFromTop = pan.scrollTop;
		const distFromBot = pan.scrollTopMax - pan.scrollTop;

		const viewableHeight = pan.clientHeight;
		const wantContentHeight = viewableHeight + pxBufferBothSides;

		const wantFetchStart = distFromTop < pxBeforeLoad && scrollDiff <= 0;
		const wantFetchEnd = distFromBot < pxBeforeLoad && scrollDiff >= 0;

		const canLoadBeforeStart = holdIdStart > fetchIdMin;
		const canLoadAfterEnd = holdIdEnd < fetchIdMax;

		if (wantFetchStart && canLoadBeforeStart) {
			const count = Math.min(holdIdStart - fetchIdMin, fetchCount);
			const from = holdIdStart - count;
			console.log("want start", from, count);
			await load(from, count);
			return false;
		} else if (wantFetchEnd && canLoadAfterEnd) {
			const from = holdIdEnd + 1;
			const count = Math.min(fetchIdMax - holdIdEnd, fetchCount);
			console.log("want end", from, count);
			await load(from, count);
			return false;
		}

		await tick();

		const hasContentHeight = pan.scrollHeight;
		const wantedContentHeigthReached =
			hasContentHeight >= wantContentHeight;
		if (!wantedContentHeigthReached) {
			throw Error("Why so sleepy?");
		}
		return true;
	}

	async function load(from, count) {
		await sleep(100);
		let fElems = dummies()
			.linq()
			.skip(from)
			.take(count)
			.toArray();

		await applyElements(fElems, from);
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

	async function tryTrimEnd() {
		if (elems.length <= nItemsToRemove) return;
		await tick();
		let childList = pan.querySelectorAll(".scrollPane > .lazyListElement");
		let nThChild = childList[childList.length - nItemsToRemove]; // -1 ??
		// The top of the element within our list (unscrolled)
		let topStaticOffset = nThChild.offsetTop - nThChild.offsetHeight;
		// The top of the element without our list (with scroll offset)
		let topCurrentOffset = topStaticOffset - pan.scrollTop;

		// Check if the top of our item is more than nThItemDistanceToView
		// below the bottom of our view
		if (topCurrentOffset > pan.offsetHeight + nThItemDistanceToView) {
			console.log(
				"trim end requested",
				topCurrentOffset,
				nThItemDistanceToView,
				pan.offsetHeight
			);
			elems = elems.slice(0, elems.length - nItemsToRemove); // modification is at the end => safe
			holdIdEnd = holdIdStart + elems.length;
			console.log("After trim end", holdIdStart, holdIdEnd);
		}
	}

	async function tryTrimStart() {
		if (elems.length <= nItemsToRemove) return;
		await tick();
		let childList = pan.querySelectorAll(".scrollPane > .lazyListElement");
		let nThChild = childList[nItemsToRemove];
		// The bottom of the element within our list (unscrolled)
		let bottomStaticOffset = nThChild.offsetTop;
		// The top of the element without our list (with scroll offset)
		let bottomCurrentOffset = bottomStaticOffset - pan.scrollTop;

		// Check if the bottom of our item is more than nThItemDistanceToView
		// above the top of our view
		if (bottomCurrentOffset < 0 - nThItemDistanceToView) {
			console.log(
				"trim start requested",
				bottomCurrentOffset,
				nThItemDistanceToView,
				pan.offsetHeight
			);
			await modifyElems(elems.slice(nItemsToRemove)); // mofification at start => helper
			holdIdStart = holdIdEnd - elems.length;
			console.log("After trim start", holdIdStart, holdIdEnd);
		}
	}

	async function applyElements(newElems, from) {
		// TODO:not sure, but I think add + trim could be done in one step
		if (from === holdIdEnd + 1) {
			// This case adds elements at the end => trim start
			elems = [...elems, ...newElems]; // modification is at the end => safe
			holdIdEnd += newElems.length;
			await tryTrimStart();
		} else if (from + newElems.length == holdIdStart) {
			// This case adds elements at the start => trim end
			await modifyElems([...newElems, ...elems]); // mofification at start => helper
			holdIdStart -= newElems.length;
			await tryTrimEnd();
		} else {
			// case when jumping to non-adjacent blocks
			// OR: overlapping case, but that should be trimmed to one of the two top one (TODO)
			throw new Error("not impl");
		}
	}

	onMount(() => {
		start_fill();
	});
</script>

<div class="lazyList" bind:this="{pan}" on:scroll="{handle_scroll}">
	<div class="scrollPane">
		{#each elems as data}
			<div class="lazyListElement">
				<slot {data} />
			</div>
		{/each}
	</div>
</div>

<style>
	.lazyList {
		border: 1px black solid;
		height: 75vh;
		overflow-x: hidden;
		overflow-y: scroll;
	}

	.scrollPane {
	}
</style>
