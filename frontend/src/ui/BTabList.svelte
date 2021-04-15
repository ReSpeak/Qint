<script lang="ts">
	import { setContext } from "svelte";
	import { writable } from "svelte/store";
	import { contextKey } from "./tabList";
	import type { TabListContext } from "./tabList";

	export let _class: string = "";
	export let _style: string = "";
	let activeId: number = 0;
	let tryRestoreTab: string | undefined = undefined;
	const indexStore = writable(activeId);
	$: indexStore.set(activeId);
	let idCounter = 0;
	let items: [number, string][] = [];
	let context: TabListContext = {
		activeId: indexStore,
		registerPanel,
		unregisterPanel,
	};
	
	setContext(contextKey, context);

	function registerPanel(title: string) {
		const tabId = idCounter++;
		items = [...items, [tabId, title]];
		if (tryRestoreTab === title) {
			activeId = tabId;
		}
		return tabId;
	}

	function unregisterPanel(id: number) {
		const remIndex = items.findIndex((item) => item[0] === id);
		if (remIndex >= 0) {
			let removedItem = items.splice(remIndex, 1)[0];
			items = items;
			if (activeId === id && items.length > 0) {
				activeId = items[Math.min(remIndex, items.length - 1)][0];
				tryRestoreTab = removedItem[1];
			}
		}
	}
</script>

<svelte:options immutable={true} />
<div class="tabList {_class}" style={_style}>
	<div class="tabs">
		<ul>
			{#each items as [id, title]}
				<li class:is-active={id === activeId}>
					<!-- svelte-ignore a11y-missing-attribute -->
					<a
						on:click={() => {
							activeId = id;
							tryRestoreTab = undefined;
						}}>
						{title}
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

	.tabList {
		display: grid;
		grid-template-rows: max-content 1fr;
	}

	.tabBody {
		padding-top: 1.5em;
		overflow: hidden;
	}

	a:hover {
		text-decoration: none;
	}

	.tabs {
		margin-bottom: 0;
	}
	.tabs li.is-active a {
		color: $main-blue;
		border-bottom-color: $main-blue;
	}
</style>
