<script lang="typescript">
	import { sleep, assert } from "../util";
	import { tick, onMount } from "svelte";
	import { writable } from 'svelte/store';
	import * as svst from 'svelte/store';
	import { ListFetchDir, FetchResult } from "./lazyList";

	// Dummy class to have nice typing for our 'generic' parameter T which
	// represents the element type.
	class T {}

	let canLoadAfterEnd: boolean = true;
	let canLoadBeforeStart: boolean = true;

	export function clear() {
		elems = [];
		canLoadAfterEnd = false;
		canLoadBeforeStart = false;
	}

	export function sourceChanged(dir: ListFetchDir) {
		switch (dir) {
		case ListFetchDir.New:
			canLoadBeforeStart = true;
			canLoadAfterEnd = true;
			break;
		case ListFetchDir.Before: canLoadBeforeStart = true; break;
		case ListFetchDir.After: canLoadAfterEnd = true; break;
		}
		start_fill();
	}

	export let fetchElements: (
		id: T | undefined,
		dir: ListFetchDir
	) => Promise<FetchResult<T>> = undefined as any;
	assert(fetchElements, "No fetch function");

	// the data elements held by this list
	let elems: T[] = [];

	let pxBeforeLoad = 300; // TODO? could be adjusted dynamically
	// The amout of items that will be removed when out of view
	let nItemsToRemove = 25;
	// How far the nThItemsToRemove has to be out of view to remove the batch
	let nThItemDistanceToView = 400;

	// Require the minimum distance before deleting an item to be higher
	// than the minimum size the list wants to buffer.
	// Otherwise we might end in an loop of adding and removing a side.
	assert(nThItemDistanceToView > pxBeforeLoad, "Distance to delete must be greater than distance to load");

	declare let holdIdStart: T | undefined;
	declare let holdIdEnd: T | undefined;
	// the lowest and highest _included_ id currently in the list
	$: holdIdStart = elems.length !== 0 ? elems[0] : undefined;
	$: holdIdEnd = elems.length !== 0 ? elems[elems.length - 1] : undefined;

	// The holding list element which has the scrollbar
	let pan: HTMLElement;
	// Utility holder to calculate `scrollDiff`
	let lastScrollPos: number = 0;
	// In which direction and how far the content has scrolled since last check
	// >0 down, <0 up
	let scrollDiff = 0;
	// prevent async weirdness by only allowing one async task
	let fetchTask: Promise<void> | undefined;

	function handle_scroll(e: MouseEvent) {
		// console.log(
		// 	pan.scrollHeight, // complete content
		// 	pan.scrollTop,    // current scroll position
		// 	pan.scrollTopMax, // max scoll position
		// 	pan.offsetHeight, // container height
		// 	pan.clientHeight, // inner view height (after subtracting border/padding)
		// );
		// ! ELEM.offsetTop   // is the bottom of a element measured from the top of the container
		// ! clientHeight + scrollTopMax === scrollHeight
		scrollDiff = pan.scrollTop - lastScrollPos;
		lastScrollPos = pan.scrollTop;

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
			if (i === loadMaxBeforeError) throw Error("yah, thats a loop");
			if (await fill_body()) break;
		}
		console.log("Task done");
		fetchTask = undefined;
	}

	/**
	 * Will repeatedly fetch list chunks from the source until no more elements
	 * can be fetched or the list is full enough.
	 * @returns true when the list is satisfied with loading data
	 */
	async function fill_body(): Promise<boolean> {
		if (!canLoadAfterEnd && !canLoadBeforeStart) return true;

		if (holdIdStart === undefined || holdIdEnd === undefined) {
			await load(undefined, ListFetchDir.New);
			return false;
		}

		const distFromTop = pan.scrollTop;
		const pan_scrollTopMax = pan.scrollHeight - pan.clientHeight;
		const distFromBot = pan_scrollTopMax - pan.scrollTop;

		const wantFetchStart = distFromTop < pxBeforeLoad && scrollDiff <= 0;
		const wantFetchEnd = distFromBot < pxBeforeLoad && scrollDiff >= 0;

		if (wantFetchStart && canLoadBeforeStart) {
			console.log("want start", holdIdStart);
			await load(holdIdStart, ListFetchDir.Before);
			return false;
		} else if (wantFetchEnd && canLoadAfterEnd) {
			console.log("want end", holdIdEnd);
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
	async function load(from: T, dir: ListFetchDir.After | ListFetchDir.Before): Promise<void>;
	async function load(from: undefined | T, dir: ListFetchDir.New): Promise<void>;
	async function load(from: T | undefined, dir: ListFetchDir): Promise<void> {
		assert(dir === ListFetchDir.New || from !== undefined, "Invalid load request. from:", from, "dir:", dir);
		const result = await fetchElements(from, dir);
		assert(result, "result from fetch is not valid");
		console.log("From fetch: ", result);

		if (dir === ListFetchDir.Before) {
			if(result.items.length === 0) assert(!result.canLoadBeforeStart, "Empty fetch result, but can still load", dir, result);
			canLoadBeforeStart = result.canLoadBeforeStart;
		} else if (dir === ListFetchDir.After) {
			if(result.items.length === 0) assert(!result.canLoadAfterEnd, "Empty fetch result, but can still load", dir, result);
			canLoadAfterEnd = result.canLoadAfterEnd;
		} else {
			if(result.items.length === 0) assert(!result.canLoadBeforeStart && !result.canLoadAfterEnd, "Empty fetch result, but can still load", dir, result);
			canLoadBeforeStart = result.canLoadBeforeStart;
			canLoadAfterEnd = result.canLoadAfterEnd;
		}
		await applyElements(result.items, dir);
	}

	/**
	 * Utility method to replace the current list with a new list without
	 * changing the scroll position.
	 */
	async function modifyElems(newElems: T[]) {
		const lastScrollHeight = pan.scrollHeight;
		const lastScrollTop = pan.scrollTop;
		console.log("Before change scrollHeight", lastScrollHeight, ", scrollTop", pan.scrollTop);
		elems = newElems;
		await tick();
		console.log("In change scrollTop", pan.scrollTop);
		const scrollAdjust = pan.scrollHeight - lastScrollHeight;
		pan.scrollTop = lastScrollTop + scrollAdjust;
		lastScrollPos += scrollAdjust;
		console.log("After change scrollHeight", pan.scrollHeight, ", scrollTop", pan.scrollTop, ", scrollAdjust", scrollAdjust);
	}

	/**
	 * Checks if a block at the end of the list is far enough out of view and
	 * removes it.
	 */
	async function tryTrimEnd() {
		if (elems.length <= nItemsToRemove) return;
		await tick();
		let childList = pan.querySelectorAll<HTMLElement>(".scrollPane > .lazyListElement");

		let nThChild = childList[childList.length - nItemsToRemove];
		// The top of the element within our list (unscrolled)
		let topStaticOffset = nThChild.offsetTop - nThChild.offsetHeight;
		// The top of the element without our list (with scroll offset)
		let topCurrentOffset = topStaticOffset - pan.scrollTop;

		console.log("tryTrimEnd", childList, nThChild, topStaticOffset, topCurrentOffset, ">", pan.offsetHeight + nThItemDistanceToView);

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
			canLoadAfterEnd = true;
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
		let childList = pan.querySelectorAll<HTMLElement>(".scrollPane > .lazyListElement");
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
			canLoadBeforeStart = true;
			console.log("After trim start", holdIdStart, holdIdEnd);
		}
	}

	/**
	 * Appends/Prepends or replaces the list with the new passed list.
	 */
	async function applyElements(newElems: T[], dir: ListFetchDir) {
		// TODO:not sure, but I think add + trim could be done in one step
		switch (dir) {
			case ListFetchDir.After:
				// This case adds elements at the end => trim start
				elems = [...elems, ...newElems]; // modification is at the end => safe
				await tryTrimStart();
				break;

			case ListFetchDir.Before:
				// This case adds elements at the start => trim end
				await modifyElems([...newElems, ...elems]); // mofification at start => helper
				await tryTrimEnd();
				break;

			case ListFetchDir.New:
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
		{#each elems as item}
			<div class="lazyListElement">
				<slot {item} />
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

	.lazyListElement {
		/*display: contents;*/
	}
</style>
