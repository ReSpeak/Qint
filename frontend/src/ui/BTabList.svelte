<script lang="typescript">
	import { setContext } from "svelte";
	import { writable } from "svelte/store";
	import { contextKey } from "./tabList";
	import type { TabListContext } from "./tabList";

	export let activeIndex: number = 0;
	const indexStore = writable(activeIndex);
	$: indexStore.set(activeIndex);
	let items: string[] = [];
	let context: TabListContext = {
		activeIndex: indexStore,
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
				<li class:is-active={index === activeIndex}>
					<!-- svelte-ignore a11y-missing-attribute -->
					<a
						on:click={() => {
							activeIndex = index;
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

<style lang="scss">
	@import "./style/global_mixin";

	.tabBody {
		padding: 0.5em;
	}

	a:hover {
		text-decoration: none;
	}

	.tabs li.is-active a {
		color: $main-blue;
		border-bottom-color: $main-blue;
	}
</style>
