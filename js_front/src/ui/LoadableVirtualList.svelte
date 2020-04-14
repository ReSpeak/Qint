<script>
	// Based on https://github.com/sveltejs/svelte-virtual-list/blob/master/VirtualList.svelte
	import { onMount, tick } from 'svelte';

	// props
	// `true` if the view starts at the top and scrolls to the bottom,
	// `false` if the view starts at the bottom.
	export let startIsTop = true;
	// A function that takes a boolean, `true` if we need more data at the start,
	// `false` if we need more data at the end.
	export let loadMore;
	export let maxItems = 500; // Don't crash the browser if something goes havoc

	// read-only, but visible to consumers via bind:items
	export let items = [];

	// local state
	let rows;
	let viewport;
	let contents;
	let viewport_height = 0;
	let mounted;
	// `true` if `items` is at the start and no more items can be loaded.
	let isAtStart = true;
	// `true` if `items` is at the end and no more items can be loaded.
	let isAtEnd = false;

	// The promise for loading items
	let loadingStart;
	let loadingEnd;
	let arrowHidden = true;
	let lastScrollTop = 0;

	let top = 0;
	let bottom = 0;

	// Refresh if something changes
	$: if (mounted) loadData(viewport_height);

	async function loadData() {
		// Load as long as necessary
		while (true) {
			await tick(); // wait until the DOM is up to date

			if (items.length >= maxItems)
				return;
			// Already loading
			if (loadingStart || loadingEnd)
				return;

			const { scrollTop } = viewport;
			// Invisible content around the viewport
			const content_buffer_start = scrollTop - top;
			const content_buffer_end = Math.max(0, contents.offsetHeight - scrollTop - viewport_height - bottom);

			// Check if we need to load more
			if (!isAtStart && content_buffer_start < viewport_height) {
				loadingStart = loadMore(true).finally(() => loadingStart = undefined);
				const newItems = await loadingStart;
				if (newItems) {
					items = [...newItems, ...items];

					// Prevent jumping
					await tick();

					let new_height = 0;
					for (let i = 0; i < newItems.length; i++)
						new_height += rows[i].offsetHeight;

					let rest = 0;
					if (new_height > top)
						rest = new_height - top;
					top = Math.max(0, top - new_height);
					// TODO Stutters here sometimes because we interupt the smooth scrolling
					viewport.scrollTo(0, scrollTop + rest);
				} else {
					isAtStart = true;
				}
			} else if (!isAtEnd && content_buffer_end < viewport_height) {
				loadingEnd = loadMore(false).finally(() => loadingEnd = undefined);
				const newItems = await loadingEnd;
				if (newItems) {
					items = [...items, ...newItems];

					// Wait until loaded
					await tick();

					let new_height = 0;
					for (let i = items.length - newItems.length; i < items.length; i++)
						new_height += rows[i].offsetHeight;

					let rest = 0;
					if (new_height > bottom)
						rest = new_height - bottom;
					bottom = Math.max(0, bottom - new_height);
				} else {
					isAtEnd = true;
				}
			} else {
				return;
			}
		}
	}

	async function handle_scroll(event) {
		const { clientHeight, scrollHeight, scrollTop } = viewport;
		// Show or hide return button
		if ((scrollTop < lastScrollTop) == startIsTop
			&& scrollTop != 0 && scrollHeight - scrollTop != clientHeight) {
			arrowHidden = false;
		} else {
			arrowHidden = true;
		}
		lastScrollTop = scrollTop;

		// Already loading, update nothing
		if (loadingStart || loadingEnd)
			return;

		await loadData();

		// Remove excessive items
		let i = 0;
		let y = 0;
		const maxTop = scrollTop - top - 2 * viewport_height;

		while (i < items.length) {
			const row_height = rows[i].offsetHeight;
			if (y + row_height >= maxTop)
				break;

			y += row_height;
			i += 1;
		}

		const start = i;
		top += y;

		const minTop = scrollTop - top + 3 * viewport_height;
		while (i < items.length) {
			y += rows[i].offsetHeight;
			i += 1;

			if (y > minTop)
				break;
		}

		const end = i + 1;
		let rest = 0;
		while (i < items.length) {
			rest += rows[i].offsetHeight;
			i += 1;
		}

		// Clamp top and bottom
		const maxSize = 50 * viewport_height;
		bottom = Math.min(bottom + rest, maxSize);
		if (top > maxSize) {
			rest = top - maxSize;
			top = maxSize;
			viewport.scrollTo(0, scrollTop - rest);
		}

		if (start > 0)
			isAtStart = false;
		if (end < items.length)
			isAtEnd = false;
		items = items.slice(start, end);
	}

	function scrollReturn() {
		if (startIsTop)
			viewport.scrollTo(0, 0);
		else
			viewport.scrollTo(0, viewport.scrollHeight);
	}

	// trigger initial refresh
	onMount(() => {
		rows = contents.getElementsByTagName('svelte-virtual-list-row');
		mounted = true;
	});
</script>

<svelte-virtual-list>
{#if !startIsTop}
	<slot name="returnArrow"><button class="arrow-down" class:arrowHidden on:click={scrollReturn}><div></div></button></slot>
{/if}
<svelte-virtual-list-viewport
	bind:this={viewport}
	bind:offsetHeight={viewport_height}
	on:scroll={handle_scroll}
>
	{#if loadingStart}
		<slot name="loading"><div>Loading…</div></slot>
	{/if}
	<svelte-virtual-list-contents
		bind:this={contents}
		style="padding-top: {top}px; padding-bottom: {bottom}px;"
	>
		{#each items as item}
			<svelte-virtual-list-row>
				<slot {item}>Missing template</slot>
			</svelte-virtual-list-row>
		{/each}
	</svelte-virtual-list-contents>
	{#if loadingEnd}
		<slot name="loading"><div>Loading…</div></slot>
	{/if}
</svelte-virtual-list-viewport>
{#if startIsTop}
	<slot name="returnArrow"><button class="arrow-up" class:arrowHidden on:click={scrollReturn}><div></div></button></slot>
{/if}
</svelte-virtual-list>

<style>
	svelte-virtual-list {
		display: block;
		position: relative;
	}

	svelte-virtual-list-viewport {
		overflow-y: auto;
	}

	.arrow-down, .arrow-up {
		position: absolute;
		right: 2em;
		display: inline-block;
		background: #ccc;
		border-radius: 100%;
		padding: 0.8em;
		border: none;
		cursor: pointer;

		transition-duration: 0.2s;
		transition-property: all;
	}

	.arrow-down:hover, .arrow-up:hover {
		background: #eee;
	}

	.arrow-down {
		top: 1.5em;
	}

	.arrow-up {
		bottom: 1.5em;
	}

	.arrow-down.arrowHidden {
		top: -5em;
	}

	.arrow-up.arrowHidden {
		bottom: -5em;
	}

	.arrow-down > div, .arrow-up > div {
		border-left: 2px solid #222;
		border-top: 2px solid #222;
		width: 1em;
		height: 1em;
	}

	.arrow-down > div {
		transform: rotate(-135deg) translate(20%, 20%);
	}

	.arrow-up > div {
		transform: rotate(45deg) translate(20%, 20%);
	}
</style>