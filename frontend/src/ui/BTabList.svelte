<script lang="typescript">
	import { setContext } from "svelte";
	import { writable } from "svelte/store";
	import { contextKey } from "./tabList";
	import type { TabListContext } from "./tabList";

	let items: string[] = [];
	let activeIndex = writable(0);
	let context: TabListContext = {
		activeIndex,
		registerPanel,
	};
	setContext(contextKey, context);

	function registerPanel(title: string) {
		items = [...items, title];
		return items.length - 1;
	}
</script>

<svelte:options immutable={true} />
<div class="tabList">
	<div class="tabs">
		<ul>
			{#each items as item, index}
				<li class:is-active={index === $activeIndex}>
					<!-- svelte-ignore a11y-missing-attribute -->
					<a
						on:click={() => {
							$activeIndex = index;
						}}>
						{item}
					</a>
				</li>
			{/each}
		</ul>
	</div>
	<div class="tabBody">
		<slot />
	</div>
</div>

<style>
	.tabBody {
		padding: 0.5em;
	}

	.tabs li.is-active a {
		color: #0dead0;
		border-bottom-color: #0dead0;
	}
</style>
