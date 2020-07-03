<script>
	import { sleep, assert } from "../util";
	import { tick, onMount } from "svelte";
	import { ListFetchDir, ListEmpty } from "./lazyList";

	// the lowest and highest id that could be retrieved from the source
	export let fetchIdMin = undefined; // T
	export let fetchIdMax = undefined; // T
	export let compare = (a, b) => {
		return a - b;
	}; // (a, b) => -1 | 0 | +1
	export let fetchElements; // (id, dir) => { elements: T[], hasEnd: boolean }
	//export let afterFetch = undefined; // () => any
	export let startIsTop = true;
	assert(fetchElements, "No fetch function");

	// A small hack to fetch new data when min/max changes
	$: start_fill(fetchIdMin);
	$: start_fill(fetchIdMax);

	// the data elements held by this list
	let elems = [];

	let pxBeforeLoad = 100; // TODO? could be adjusted dynamically
	// The amout of items that will be removed when out of view
	let nItemsToRemove = 25;
	// How far the nThItemsToRemove has to be out of view to remove the batch
	let nThItemDistanceToView = 200;

	// Require the minimum distance before deleting an item to be higher
	// than the minimun size the list wants to buffer.
	// Otherwise we might end in an loop of adding and removing a side.
	assert(nThItemDistanceToView > pxBeforeLoad);

	// the lowest and highest _included_ id currently in the list
	$: holdIdStart = elems.length !== 0 ? elems[0] : undefined;
	$: holdIdEnd = elems.length !== 0 ? elems[elems.length - 1] : undefined;

	// The holding list element which has the scrollbar
	let pan;
	// Utility holder to calculate `scrollDiff`
	let lastScollPos = undefined;
	// In which direction and how far the content has scrolled since last check
	// >0 down, <0 up
	let scrollDiff = 0;
	// prevent async weirness by only allowing one async task
	let fetchTask;

	function handle_scroll(e) {
		// console.log(
		// 	pan.scrollHeight, // complete content
		// 	pan.scrollTop,    // current scroll position
		// 	pan.scrollTopMax, // max scoll position
		// 	pan.offsetHeight, // container height
		// 	pan.clientHeight, // inner view height (after subtracting border/padding)
		// );
		// ! ELEM.offsetTop   // is the bottom of a element measured from the top of the container
		// ! clientHeight + scrollTopMax == scrollHeight
		if (lastScollPos !== undefined) {
			scrollDiff = pan.scrollTop - lastScollPos;
			//console.log("diff", scrollDiff);
		}
		lastScollPos = pan.scrollTop;

		start_fill();
	}

	function start_fill() {
		if (fetchTask) {
			return;
		}
		fetchTask = fill_loop();
	}

	async function fill_loop() {
		const loadMaxBeforeError = 50;
		for (let i = 0; i <= loadMaxBeforeError; i++) {
			if (i == loadMaxBeforeError) throw Error("yah, thats a loop");
			if (await fill_body()) break;
		}
		console.log("Task done");
		fetchTask = undefined;
	}

	/**
	 * Will repeatedly fetch list blocks from the source until no more elements
	 * can be fetched or the list is full enough.
	 * @returns true when the list is satisfied with loading data
	 */
	async function fill_body() {
		if (holdIdStart === undefined || holdIdEnd === undefined) {
			await load(undefined, ListFetchDir.New);
			return false;
		}
		// if (holdIdStart === undefined && holdIdEnd === undefined) {
		// 	await load(undefined, ListFetchDir.New);
		// 	return false;
		// }

		const distFromTop = pan.scrollTop;
		const distFromBot = pan.scrollTopMax - pan.scrollTop;

		const wantFetchStart = distFromTop < pxBeforeLoad && scrollDiff <= 0;
		const wantFetchEnd = distFromBot < pxBeforeLoad && scrollDiff >= 0;

		const canLoadBeforeStart =
			holdIdStart === undefined || compare(holdIdStart, fetchIdMin) > 0;
		const canLoadAfterEnd =
			holdIdStart === undefined || compare(holdIdEnd, fetchIdMax) < 0;

		if (wantFetchStart && canLoadBeforeStart) {
			//const count = Math.min(holdIdStart - fetchIdMin, fetchCount);
			//const from = holdIdStart - count;
			//console.log("want start", from, count);
			await load(holdIdStart, ListFetchDir.Before);
			return false;
		} else if (wantFetchEnd && canLoadAfterEnd) {
			//const from = holdIdEnd + 1;
			//const count = Math.min(fetchIdMax - holdIdEnd, fetchCount);
			//console.log("want end", from, count);
			await load(holdIdEnd, ListFetchDir.After);
			return false;
		} else {
			return true;
		}
	}

	/**
	 * Checks the direction of the request. Fetches the data from the source
	 * and applies them into the list.
	 * This will fetch one block only.
	 */
	async function load(from, dir) {
		await sleep(100);
		const elements = await fetchElements(from, dir);

		if (elements.length === 0) {
			if (dir === ListFetchDir.Before) fetchIdMin = from;
			else if (dir === ListFetchDir.After) fetchIdMax = from;
			else {
				fetchIdMin = from;
				fetchIdMax = from;
			}
			return;
		}
		await applyElements(elements, dir);
	}

	/**
	 * Utility method to replace the current list with a new list without
	 * changing the scroll position.
	 */
	async function modifyElems(newElems) {
		const lastScrollHeight = pan.scrollHeight;
		assert(Array.isArray(newElems));
		elems = newElems;
		await tick();
		var scrollDiff = pan.scrollHeight - lastScrollHeight;
		pan.scrollTop += scrollDiff;
		lastScollPos += scrollDiff;
	}

	/**
	 * Checks if a block at the end of the list is far enough out of view and
	 * removes it.
	 */
	async function tryTrimEnd() {
		if (elems.length <= nItemsToRemove) return;
		await tick();
		let childList = pan.querySelectorAll(".scrollPane > .lazyListElement");
		let nThChild = childList[childList.length - nItemsToRemove];
		// The top of the element within our list (unscrolled)
		let topStaticOffset = nThChild.offsetTop - nThChild.offsetHeight;
		// The top of the element without our list (with scroll offset)
		let topCurrentOffset = topStaticOffset - pan.scrollTop;

		// Check if the top of our item is more than nThItemDistanceToView
		// below the bottom of our view
		if (topCurrentOffset > pan.offsetHeight + nThItemDistanceToView) {
			console.log(
				"trim end",
				topCurrentOffset,
				nThItemDistanceToView,
				pan.offsetHeight
			);
			// modification is at the end => safe
			elems = elems.slice(0, elems.length - nItemsToRemove);
			holdIdEnd = elems[elems.length - 1];
			console.log("After trim end", holdIdStart, holdIdEnd);
		}
	}

	/**
	 * Checks if a block at the start of the list is far enough out of view and
	 * removes it.
	 */
	async function tryTrimStart() {
		if (elems.length <= nItemsToRemove) return;
		await tick();
		let childList = pan.querySelectorAll(".scrollPane > .lazyListElement");
		let nThChild = childList[nItemsToRemove - 1];
		// The bottom of the element within our list (unscrolled)
		let bottomStaticOffset = nThChild.offsetTop;
		// The top of the element without our list (with scroll offset)
		let bottomCurrentOffset = bottomStaticOffset - pan.scrollTop;

		// Check if the bottom of our item is more than nThItemDistanceToView
		// above the top of our view
		if (bottomCurrentOffset < 0 - nThItemDistanceToView) {
			console.log(
				"trim start",
				bottomCurrentOffset,
				nThItemDistanceToView,
				pan.offsetHeight
			);
			// mofification at start => helper
			await modifyElems(elems.slice(nItemsToRemove));
			holdIdStart = elems[0];
			console.log("After trim start", holdIdStart, holdIdEnd);
		}
	}

	/**
	 * Appends/Prepends or replaces the list with the new passed list.
	 */
	async function applyElements(newElems, dir) {
		// TODO:not sure, but I think add + trim could be done in one step
		switch (dir) {
			case ListFetchDir.After:
				// This case adds elements at the end => trim start
				elems = [...elems, ...newElems]; // modification is at the end => safe
				holdIdEnd += newElems.length;
				await tryTrimStart();
				break;

			case ListFetchDir.Before:
				// This case adds elements at the start => trim end
				await modifyElems([...newElems, ...elems]); // mofification at start => helper
				holdIdStart -= newElems.length;
				await tryTrimEnd();

			case ListFetchDir.New:
				assert(Array.isArray(newElems));
				elems = newElems;
				break;

			default:
				throw new Error("Unhandled direction case");
		}
	}

	onMount(() => {
		start_fill();
		pan.onresize = start_fill;
	});
</script>

<div class="lazyList" bind:this="{pan}" on:scroll="{handle_scroll}">
	<div class="scrollPane">
		<!-- {@debug elems} -->
		{#each elems as data}
			<div class="lazyListElement">
				<slot {data} />
			</div>
		{/each}
	</div>
</div>

<style>
	.lazyList {
		overflow-x: hidden;
		overflow-y: scroll;
		height: 100%;
	}
</style>
