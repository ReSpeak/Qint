<script lang="typescript">
	import { get } from "svelte/store";
	import Icon from "../ui/Icon.svelte";
	import TsIcon from "../ui/TsIcon.svelte";
	import FilterString from "../ui/FilterString.svelte";
	import { Channel, Client } from "./book";
	import type { ITreeNode } from "./book";
	import UiClient from "./UiClient.svelte";
	import { Connection } from "../connection";
	import { draggable, DragData } from "../ui/draggable";
	import { findParent, assert } from "../util";

	export let connection: Connection;
	export let filter: string;
	export let filterShow: boolean = true;
	export let channel: Channel;
	let selectedChat = connection.chat.selectedChat;

	let collapsed = false;
	let hovered = false;

	let isSelected: boolean = false;
	$: children = channel.children;
	$: filterShow = applyFilter(filter, channel, $children);
	// Update if a client moves in or out
	$: ownClient = updateOwnClient($children);
	$: {
		const sc = $selectedChat;
		isSelected = "Channel" in sc && sc.Channel === channel.id;
	}
	let div!: HTMLElement;

	function updateOwnClient(_children: ITreeNode[]) {
		let isOwn = false;
		let client = get(connection.ownClient);
		if (client !== undefined) {
			isOwn = client.channel === channel.id;
		}
		return isOwn;
	}

	function applyFilter(filter: string, channel: Channel, children: ITreeNode[]) {
		assert(filter != null, "fil");
		return (
			filter === "" ||
			channel.name.toLowerCase().includes(filter.toLowerCase()) ||
			children.some((c) => c.filterShow)
		);
	}

	function switchChannel() {
		connection.switchChannel(channel);
	}

	function setChat() {
		connection.chat.selectChannel(channel);
	}

	function leave(event: MouseEvent) {
		if (event.relatedTarget) {
			if (div.contains(event.relatedTarget as Node)) {
				return;
			}
		}
		hovered = false;
	}

	function dragStart(ev: CustomEvent<DragData>) {
		ev.detail.dragNode.classList.add("dragStyle");
		const channelTree = findParent(ev.detail.dragNode, ".channel-list")!;
		ev.detail.customData = channelTree;
		// TODO find correct max
		//ev.detail.maxY = channelTree.clientHeight;
		ev.detail.lockX = true;
	}

	function dragDrop(ev: CustomEvent<DragData>) {
		ev.detail.dragNode.classList.remove("dragStyle");
		const hoverOpt: HTMLElement[] = [
			...ev.detail.customData.querySelectorAll(":hover"),
		].reverse();
		const dropTarget = hoverOpt.find((x) => x.dataset.type === "channel");
		console.log(hoverOpt, dropTarget);
		if (dropTarget !== undefined) {
			const rect = dropTarget.getBoundingClientRect();
			let clickY = ev.detail.mouseEvent.clientY - rect.top;
			let clickPerc = clickY / (rect.bottom - rect.top);
			let target = connection.book.getChannel(Number(dropTarget.dataset.key))!;
			// < 0.25      : Dropped in the upper quarter
			// 0.25 - 0.75 : Dropped in the middle half
			// > 0.75      : Dropped in the lower quarter

			if (clickPerc < 0.25) {
				// Case A: Dropped TOP
				//      => Insert in same parent as target, steal order of target
				connection.moveChannel(channel.id, target.parent, target.order);
			} else if (clickPerc < 0.75) {
				// Case B: Dropped MIDDLE
				//      => Target is the new parent, order 0 since it's the first child now
				connection.moveChannel(channel.id, target.id, 0);
			} else {
				// Dropped BOTTOM
				if (get(target.children).length > 0) {
					// Case C: Channel HAS child
					//      => Same as middle
					connection.moveChannel(channel.id, target.id, 0);
				} else {
					// Case D: Channel NO child
					//      => Place below target, parent same as target, order is target
					connection.moveChannel(channel.id, target.parent, target.id);
				}
			}

			console.log("Would drop", channel.id, "to", dropTarget.dataset.key, "at", clickPerc);
		}
	}
</script>

<li class="container" class:hidden={!filterShow} class:collapsed>
	<div
		bind:this={div}
		on:mouseover={() => (hovered = true)}
		on:mouseout={leave}
		class="hoverDummy">
		<div
			class="innerContainer"
			class:ownClient
			class:isSelected
			use:draggable
			on:svddrag={dragStart}
			on:svddrop={dragDrop}
			data-type="channel"
			data-key={channel.id}>
			<button
				class="button collapseButton noBut"
				on:click={() => (collapsed = !collapsed)}
				class:haschildren={$children.length !== 0}>
				<Icon name="chevron-right{collapsed ? '' : ' mdi-rotate-90'}" />
				<TsIcon type="channel" source={channel} {connection} />
			</button>
			<span class="nameBox" on:click={setChat}>
				<FilterString {filter} content={channel.name} />
			</span>
			<span class="icons">
				<button class="button noBut" on:click={switchChannel}>
					<Icon name="shoe-print" />
				</button>
			</span>
		</div>
		{#if hovered}
			<div class="hover menu" style="top: {div.getBoundingClientRect().top}px;">
				<div class="corner" />
				{channel.name}
			</div>
		{/if}
	</div>
	<ul class="menu-list">
		{#each $children as child (child.key)}
			{#if child instanceof Channel}
				<svelte:self
					{connection}
					{filter}
					channel={child}
					bind:filterShow={child.filterShow} />
			{:else if child instanceof Client}
				<UiClient {connection} {filter} client={child} bind:filterShow={child.filterShow} />
			{:else}
				{@debug child}
			{/if}
		{/each}
	</ul>
</li>

<style lang="scss">
	@import "./tree";

	.noBut {
		@include noBut;
	}

	.collapseButton {
		justify-content: start;
		display: grid;

		> :global(.icon) {
			transition: all 0.1s;
			grid-row: 1;
			grid-column: 1;
			margin: 0;
		}

		> :global(*) {
			overflow: hidden;
			text-overflow: ellipsis;
		}
	}

	.collapseButton > :global(*:first-child) {
		opacity: 0;
	}
	.collapseButton.haschildren:hover > :global(*:first-child) {
		opacity: 1;
	}
	.collapseButton.haschildren:hover > :global(*:last-child) {
		opacity: 0;
	}

	.menu-list li ul {
		margin: 0 0 0 0.5em;
		padding-left: 0.5em;
	}

	.collapsed .menu-list {
		display: none;
	}

	.collapsed .innerContainer .nameBox {
		color: mix($text, $background, 60%);
	}
</style>
