<script lang="ts">
	import { CustomIntersectionObserver, ListFetchDir } from "./uiLazyList";
	import type { FetchResult } from "./uiLazyList";
	import { assert, binarySearchByKey } from "../../util";
	import { createEventDispatcher, tick, onMount } from "svelte";
	import ResizeObserver from "resize-observer-polyfill";
	import debug from "debug";
	const log = debug("LL");
	const dispatch = createEventDispatcher<{ viewchanged: { first?: T; last?: T } }>();

	// Dummy class to have nice typing for our 'generic' parameter T which
	// represents the element type.
	type T = any;

	// The golden handbook for js/css:
	// - pan.scrollHeight, // complete content
	// - pan.scrollTop,    // current scroll position
	// - pan.scrollTopMax, // max scoll position
	// - pan.offsetHeight, // container height
	// - pan.clientHeight, // inner view height (after subtracting border/padding)
	// ! ELEM.offsetTop    // is the top of a element measured from the top of the first position:relative parent.
	// ! clientHeight + scrollTopMax === scrollHeight

	// *** State+Export variables ***

	export let enableFetching: boolean = true;
	export let suggestJumpStart: boolean = false;
	export let suggestJumpEnd: boolean = false;
	export let notifyViewChanged: boolean = false;
	let canLoadBeforeStart: boolean = true;
	let canLoadAfterEnd: boolean = true;
	let showJumpStart: boolean;
	let showJumpEnd: boolean;
	let loadAnchored: ListFetchDir | undefined = undefined;
	let lastViewStart: number | undefined;
	let lastViewEnd: number | undefined;
	let visObs: CustomIntersectionObserver;

	// the data elements held by this list
	let elems: T[] = [];

	const pxBeforeLoad = 500; // TODO? could be adjusted dynamically
	/** The minimum amout of items that must be at least `minPxDistanceToRemove`
	 * will be removed when out of view */
	const minItemsToRemove = 20;
	/** How far the item at index `minItemsToRemove` has to be out of view to be removed */
	const minPxDistanceToRemove = 1500;
	// The holding list element which has the scrollbar
	let pan: HTMLElement;
	let scrollPane: HTMLElement;
	let mounted = false;
	// prevent async weirdness by only allowing one async task
	let fetchTask: Promise<void> | undefined;

	// *** Export functions ***

	export function clear(): void {
		elems = [];
		visObs.clear();
		canLoadAfterEnd = false;
		canLoadBeforeStart = false;
		showJumpStart = false;
		showJumpEnd = false;
		lastViewStart = undefined;
		lastViewEnd = undefined;
	}

	export function sourceChanged(dir: ListFetchDir, anchor?: ListFetchDir): void {
		loadAnchored = anchor;
		switch (dir) {
			case ListFetchDir.New:
				clear();
				canLoadBeforeStart = true;
				canLoadAfterEnd = true;
				break;
			case ListFetchDir.Before:
				canLoadBeforeStart = true;
				break;
			case ListFetchDir.After:
				canLoadAfterEnd = true;
				break;
		}
		start_fill();
	}

	export function jumpTo(dir: ListFetchDir, target?: T): void {
		switch (dir) {
			case ListFetchDir.Before:
				if (!canLoadBeforeStart) {
					scrollToStart();
					break;
				}
				sourceChanged(ListFetchDir.New, ListFetchDir.Before);
				break;

			case ListFetchDir.After:
				if (!canLoadAfterEnd) {
					scrollToEnd();
					break;
				}
				sourceChanged(ListFetchDir.New, ListFetchDir.After);
				break;

			case ListFetchDir.New:
				assert(target, "target must be given when jumping to a new item");
				// TODO
				break;
		}
	}

	type fetchFun<TL = any> = (id: TL | undefined, dir: ListFetchDir) => Promise<FetchResult<TL>>;
	export let fetchElements: fetchFun;

	// Require the minimum distance before deleting an item to be higher
	// than the minimum size the list wants to buffer.
	// Otherwise we might end in an loop of adding and removing a side.
	assert(
		minPxDistanceToRemove > pxBeforeLoad,
		"Distance to delete must be greater than distance to load"
	);
	assert(minItemsToRemove >= 1, "Minimum items to remove must be >=1");

	// *** Private functions ***

	function setPanScrollTop(scrollTop: number) {
		// Why?, you might ask. Because `pan.scollTop = x;` will cause svelte to invalidate `pan`.
		const panl = pan;
		panl.scrollTop = scrollTop;
	}
	function scrollToStart() {
		if (!mounted) return;
		setPanScrollTop(0);
	}
	function scrollToEnd() {
		if (!mounted) return;
		setPanScrollTop(pan.scrollHeight - pan.clientHeight);
	}
	function getFirstElem() {
		return elems.length !== 0 ? elems[0] : undefined;
	}
	function getLastElem() {
		return elems.length !== 0 ? elems[elems.length - 1] : undefined;
	}

	function getHtmlElements(): ArrayLike<HTMLElement> {
		const childList = (scrollPane.children as any) as ArrayLike<HTMLElement>;
		assert(childList.length === elems.length, "HTML node count does not match elements count");
		return childList;
	}

	function start_fill() {
		if (fetchTask || !mounted || pan === null) {
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
		fetchTask = undefined;
	}

	/**
	 * Will repeatedly fetch list chunks from the source until no more elements
	 * can be fetched or the list is full enough.
	 * @returns true when the list is satisfied with loading data
	 */
	async function fill_body(): Promise<boolean> {
		if ((!canLoadAfterEnd && !canLoadBeforeStart) || !enableFetching || pan === null)
			return true;

		const holdIdStart = getFirstElem();
		const holdIdEnd = getLastElem();
		if (holdIdStart === undefined || holdIdEnd === undefined) {
			await load(ListFetchDir.New);
			return false;
		}

		const distFromTop = pan.scrollTop;
		const pan_scrollTopMax = pan.scrollHeight - pan.clientHeight;
		const distFromBot = pan_scrollTopMax - pan.scrollTop;

		const wantFetchStart = distFromTop <= pxBeforeLoad;
		const wantFetchEnd = distFromBot <= pxBeforeLoad;

		if (wantFetchStart && canLoadBeforeStart) {
			log("want start %o", holdIdStart);
			await load(ListFetchDir.Before, holdIdStart);
			return false;
		} else if (wantFetchEnd && canLoadAfterEnd) {
			log("want end %o", holdIdEnd);
			await load(ListFetchDir.After, holdIdEnd);
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
	async function load(dir: ListFetchDir.After | ListFetchDir.Before, from: T): Promise<void>;
	// eslint-disable-next-line no-redeclare
	async function load(dir: ListFetchDir.New, from?: T): Promise<void>;
	// eslint-disable-next-line no-redeclare
	async function load(dir: ListFetchDir, from?: T): Promise<void> {
		assert(
			dir === ListFetchDir.New || from !== undefined,
			"Invalid load request. from:",
			from,
			"dir:",
			ListFetchDir[dir]
		);
		const result = await fetchElements(from, dir);
		assert(result, "result from fetch is not valid");
		log("fetchElements result %o", result);

		if (dir === ListFetchDir.Before) {
			if (result.items.length === 0)
				assert(
					!result.canLoadBeforeStart,
					"Empty fetch result, but can still load",
					dir,
					result
				);
			canLoadBeforeStart = result.canLoadBeforeStart;
		} else if (dir === ListFetchDir.After) {
			if (result.items.length === 0)
				assert(
					!result.canLoadAfterEnd,
					"Empty fetch result, but can still load",
					dir,
					result
				);
			canLoadAfterEnd = result.canLoadAfterEnd;
		} else {
			if (result.items.length === 0)
				assert(
					!result.canLoadBeforeStart && !result.canLoadAfterEnd,
					"Empty fetch result, but can still load",
					dir,
					result
				);
			canLoadBeforeStart = result.canLoadBeforeStart;
			canLoadAfterEnd = result.canLoadAfterEnd;
		}
		await applyElements(result.items, dir);
	}

	async function oneForAllReplace(newElems: T[]) {
		if (!mounted) return;

		elems = [...newElems];
		await tick();

		lockIndex = undefined;

		oneForAllReregister();
	}

	async function oneForAllPrepend(prepend: T[]) {
		if (!mounted) return;

		const childList = getHtmlElements();
		let newElems = [...prepend, ...elems];

		if (elems.length > minItemsToRemove) {
			const distFn = (e: HTMLElement) => {
				// The top of the element within our list (unscrolled)
				const topStaticOffset = e.offsetTop;
				// The top of the element without our list (with scroll offset)
				const topCurrentOffset = topStaticOffset - pan.scrollTop;
				return topCurrentOffset;
			};

			const removeDistance = pan.offsetHeight + minPxDistanceToRemove;
			const removeIndex = childList.length - minItemsToRemove;
			const res = binarySearchByKey(childList, removeDistance, distFn, 0, removeIndex + 1);

			if (res.index <= removeIndex) {
				newElems = newElems.slice(0, prepend.length + res.index);
				canLoadAfterEnd = true;
			}
		}

		visObs.clear();

		elems = newElems;
		await tick();

		if (lockIndex !== undefined) lockIndex = prepend.length + lockIndex;

		oneForAllReregister();
	}

	async function oneForAllAppend(append: T[]) {
		if (!mounted) return;

		const childList = getHtmlElements();
		let newElems = [...elems, ...append];

		if (elems.length > minItemsToRemove) {
			const distFn = (e: HTMLElement) => {
				// The bottom of the element within our list (unscrolled)
				const bottomStaticOffset = e.offsetTop + e.offsetHeight;
				// The top of the element without our list (with scroll offset)
				const bottomCurrentOffset = bottomStaticOffset - pan.scrollTop;
				return bottomCurrentOffset;
			};

			const res = binarySearchByKey(
				childList,
				-minPxDistanceToRemove,
				distFn,
				minItemsToRemove - 1,
				undefined
			);

			log("tryTrimStart %o %o", res, res.index >= minItemsToRemove);
			// We are trimming index-1 since the result item is the first element
			// that is _smaller_ than our threshold distance.
			if (res.index >= minItemsToRemove) {
				const sliceStart = res.index - 1;
				newElems = newElems.slice(sliceStart);
				if (lockIndex !== undefined) lockIndex -= sliceStart;
				canLoadBeforeStart = true;
			}
		}

		visObs.clear();

		elems = newElems;
		await tick();

		oneForAllReregister();
	}

	function oneForAllReregister() {
		triggerResizing();

		const childList = getHtmlElements();
		visObs.observeNodes(childList);
		lastViewStart = undefined;
		lastViewEnd = undefined;
	}

	/**
	 * Appends/Prepends or replaces the list with the new passed list.
	 */
	async function applyElements(newElems: T[], dir: ListFetchDir) {
		if (pan === null) return;
		switch (dir) {
			case ListFetchDir.After:
				// This case adds elements at the end => trim start
				if (newElems.length > 0) {
					await oneForAllAppend(newElems);
				}
				break;

			case ListFetchDir.Before:
				// This case adds elements at the start => trim end
				if (newElems.length > 0) {
					await oneForAllPrepend(newElems);
				}
				break;

			case ListFetchDir.New:
				if (loadAnchored === ListFetchDir.After) {
					docked = true;
				}
				await oneForAllReplace(newElems);
				if (loadAnchored === ListFetchDir.Before) {
					scrollToStart();
				}
				break;

			default:
				throw new Error("Unhandled direction case");
		}
		recheckJumpButton();
	}

	// Resize/Scroll events
	let docked = false;
	let allowRelock = true;

	let lockIndex: number | undefined;
	let lockPos: number | undefined;

	function tryGetLockElem() {
		if (lockIndex === undefined) return undefined;
		const helems = getHtmlElements();
		return helems[lockIndex];
	}

	function triggerResizing() {
		allowRelock = false;
		if (docked) {
			setPanScrollTop(pan.scrollHeight);
		} else {
			const lockElem = tryGetLockElem();
			if (lockElem !== undefined && lockPos !== undefined) {
				assert(lockElem.parentElement, "lockElem must be in DOM to jump to");
				setPanScrollTop(lockElem.offsetTop - lockPos);
				log("jumping %d %d %d", lockElem.offsetTop - lockPos, lockElem.offsetTop, lockPos);
			}
		}
	}

	function handle_scroll() {
		//console.log("scroll", "locked", lockIndex);
		if (allowRelock) {
			setLockPos();
			if (pan.scrollTop === pan.scrollHeight - pan.clientHeight) {
				//if (docked === false) log("docked");
				docked = true;
			} else {
				//if (docked === true) log("undocked");
				docked = false;
			}
			start_fill();
		}
		recheckJumpButton();
	}

	function recheckJumpButton() {
		if (elems.length === 0) {
			showJumpStart = false;
			showJumpEnd = false;
		} else {
			// Update jump up/down button visibility
			const isScrollable = pan.scrollHeight > pan.clientHeight;
			if (suggestJumpStart) {
				const isScrolledToStart = pan.scrollTop <= 0;
				showJumpStart = isScrollable && (!isScrolledToStart || canLoadBeforeStart);
			}
			if (suggestJumpEnd) {
				const isScrolledToEnd = pan.scrollTop >= pan.scrollHeight - pan.clientHeight;
				showJumpEnd = isScrollable && (!isScrolledToEnd || canLoadAfterEnd);
			}
		}
	}

	function setLockPos(index?: number) {
		if (!allowRelock) return;
		if (index !== undefined) {
			lockIndex = index;
		} else if (lockIndex === undefined) {
			return;
		}

		const lockElem = tryGetLockElem();
		if (lockElem === undefined) {
			return;
		}
		assert(lockElem.parentElement, "lockElem must be in DOM to lock to");
		lockPos = lockElem.offsetTop - pan.scrollTop;
	}

	function onIntersectionChanged(elem: IntersectionObserverEntry[]): void {
		const oldIdStart = lastViewStart;
		const oldIdEnd = lastViewEnd;
		let newLockElem: HTMLElement | undefined;
		for (const e of elem) {
			const htmlElem = e.target as HTMLElement;
			const elemId = Number(htmlElem.dataset.index);
			if (e.isIntersecting) {
				if (lastViewStart === undefined || elemId < lastViewStart) {
					newLockElem = htmlElem;
					lastViewStart = elemId;
				} else if (lastViewEnd === undefined || elemId > lastViewEnd) {
					lastViewEnd = elemId;
				}
			} else {
				if (
					e.boundingClientRect.bottom < e.rootBounds!.top &&
					(lastViewStart === undefined || elemId + 1 > lastViewStart)
				) {
					newLockElem = htmlElem.nextElementSibling
						? (htmlElem.nextElementSibling as HTMLElement)
						: htmlElem;
					lastViewStart = elemId + 1;
				} else if (
					e.boundingClientRect.top > e.rootBounds!.bottom &&
					(lastViewEnd === undefined || elemId - 1 < lastViewEnd)
				) {
					lastViewEnd = elemId - 1;
				}
			}
		}

		const startChanged = oldIdStart !== lastViewStart;
		const endChanged = oldIdEnd !== lastViewEnd;
		if (startChanged && newLockElem !== undefined) {
			const newLockIndex = Number(newLockElem.dataset.index);
			//console.log("Insersec Loc", newLockIndex, allowRelock);
			if (allowRelock) {
				setLockPos(newLockIndex);
			}
			allowRelock = true;
		}
		if (notifyViewChanged && (startChanged || endChanged)) {
			dispatch("viewchanged", {
				first: lastViewStart !== undefined ? elems[lastViewStart] : undefined,
				last: lastViewEnd !== undefined ? elems[lastViewEnd] : undefined,
			});
		}
	}

	onMount(() => {
		const resizeObserver = new ResizeObserver(
			(_entries: ResizeObserverEntry[], _observer: ResizeObserver) => {
				//console.log(entries, "locked", lockIndex);
				triggerResizing();
			}
		);
		resizeObserver.observe(pan);
		resizeObserver.observe(scrollPane);

		visObs = new CustomIntersectionObserver(onIntersectionChanged, {
			root: pan,
			threshold: 0,
		});

		mounted = true;
		return () => {
			resizeObserver.disconnect();
			visObs.disconnect();
			mounted = false;
		};
	});
</script>

<div class="lazyList">
	<button class="arrow-up" class:showJumpStart on:click={() => jumpTo(ListFetchDir.Before)}>
		<div />
	</button>
	<div class="lazyListView" bind:this={pan} on:scroll={handle_scroll}>
		<div class="scrollPane" bind:this={scrollPane}>
			{#each elems as item, index (item)}
				<div class="lazyListElement" data-index={index}>
					<slot {item} />
				</div>
			{/each}
		</div>
		{#if elems.length === 0}
			<div class="filler">
				{#if fetchTask !== undefined}
					<slot name="loading" />
				{:else}
					<slot name="empty" />
				{/if}
			</div>
		{/if}
	</div>
	<button class="arrow-down" class:showJumpEnd on:click={() => jumpTo(ListFetchDir.After)}>
		<div />
	</button>
</div>

<style lang="scss">
	.lazyList {
		position: relative;
		overflow: hidden;
	}

	.lazyListView {
		// The position: relative makes that getting offsetTop from child elements
		// will use the scrollPane element as the relative parent.
		position: relative;
		overflow-x: hidden;
		overflow-y: scroll;
		height: 100%;
	}

	// Jump start end buttons

	.arrow-down,
	.arrow-up {
		position: absolute;
		right: 2em;

		display: inline-block;
		background: #ccc;
		border-radius: 100%;
		padding: 0.8em;
		border: none;
		cursor: pointer;
		z-index: 3;

		transition-duration: 0.2s;
		transition-property: all;

		&:hover {
			background: #eee;
		}

		> div {
			border-left: 2px solid #222;
			border-top: 2px solid #222;
			width: 1em;
			height: 1em;
		}
	}

	.arrow-down {
		bottom: -5em;
		> div {
			transform: rotate(-135deg) translate(20%, 20%);
		}
	}

	.arrow-up {
		top: -5em;
		> div {
			transform: rotate(45deg) translate(20%, 20%);
		}
	}

	.showJumpStart {
		top: 1.5em;
	}

	.showJumpEnd {
		bottom: 1.5em;
	}

	// Anchoring
	// Shouldn't do much, but prevent our scrolling panel to be selected as
	// anchor since it's useless.
	.lazyListView,
	.scrollPane {
		overflow-anchor: none;
	}

	.filler {
		width: 100%;
		height: 100%;
	}
</style>
