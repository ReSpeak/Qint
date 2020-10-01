<script lang="typescript">
	import { afterUpdate } from "svelte";
	import { get } from "svelte/store";
	import Icon from "../ui/Icon.svelte";
	import TsIcon from "../ui/TsIcon.svelte";
	import FilterString from "../ui/FilterString.svelte";
	import { Channel } from "../book";
	import type { ITreeNode } from "../book";
	import UiClient from "./UiClientWrap.svelte";
	import UiChannel from "./UiChannelWrap.svelte";
	import { Connection } from "../connection";
	import { draggable, DragData } from "../ui/draggable";
	import { findParent, assert, flash } from "../util";
	import { SpacerType } from "./tree";
	import { ChannelType } from "../ts";
	import { app } from "../app";

	afterUpdate(() => flash(div));

	export let connection: Connection | undefined = undefined;
	export let server: string | undefined = undefined;
	export let filter: string;
	export let filterShow: boolean = true;
	export let filterStartFromRoot: boolean;
	export let channel: Channel;

	let collapsed = false;
	let hovered = false;
	let showId = false;
	let thisFilter = "";
	let childrenFilter = "";

	$: isSelected = $channel.isSelected;
	$: channels = channel.channels;
	$: clients = channel.clients;
	$: filterShow = applyFilter(filter, filterStartFromRoot, $channel, $channels, $clients);
	// Update if a client moves in or out
	$: ownClient = updateOwnClient($clients);

	let spacerType: SpacerType;
	let displayName: string;
	$: {
		let chanData = getDisplayName($channel);
		spacerType = chanData.type;
		displayName = chanData.name;
	}

	let div: HTMLElement;

	function updateOwnClient(_children: ITreeNode[]) {
		if (connection === undefined) return false;
		let client = get(connection.book.ownClient);
		if (client === undefined) return false;
		return client.channel === channel.id;
	}

	function applyFilter(
		filter: string,
		filterStartFromRoot: boolean,
		channel: Channel,
		channels: ITreeNode[],
		clients: ITreeNode[],
	) {
		const children = channels.concat(clients);
		assert(filter != null, "filter is null");
		if (filter === "") {
			if (showId) showId = false;
			if (childrenFilter !== filter) childrenFilter = filter;
			if (thisFilter !== filter) thisFilter = filter;
			return true;
		}
		const filterById = filter[0] === "/";
		if (filterById) {
			// Ignore filterStartFromRoot when matching by id
			const matches = channel.id.toString().includes(filter.substr(1));
			if (!showId) showId = true;
			if (childrenFilter !== filter) childrenFilter = filter;
			if (thisFilter !== filter.substr(1)) thisFilter = filter.substr(1);
			return matches || children.some((c) => c.filterShow);
		} else {
			if (showId) showId = false;
			const index = filter.indexOf("/");
			const newThisFilter = index === -1 ? filter : filter.substr(0, index);
			if (thisFilter !== newThisFilter) thisFilter = newThisFilter;
			const matches = channel.name.toLowerCase().includes(thisFilter.toLowerCase());
			if (filterStartFromRoot) {
				let newChildrenFilter = "";
				if (index !== -1) newChildrenFilter = filter.substr(index + 1);
				if (childrenFilter !== newChildrenFilter) childrenFilter = newChildrenFilter;
				return matches && (childrenFilter === "" || children.some((c) => c.filterShow));
			}
			if (childrenFilter !== filter) childrenFilter = filter;
			return matches || children.some((c) => c.filterShow);
		}
	}

	function switchChannel() {
		connection?.switchChannel(channel);
	}

	function setChat() {
		if (connection !== undefined) app.select(connection, channel);
	}

	function hover() {
		if (connection === undefined) return;
		hovered = true;
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
		if (dropTarget !== undefined && connection !== undefined) {
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
				if (get(target.channels).length > 0) {
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

	function getDisplayName(c: Channel) {
		// TODO consider special names [*spacer] --- ... -.- ___ -..
		let data = { type: SpacerType.None, name: c.name };
		if (c.parent !== 0 || c.channel_type !== ChannelType.Permanent) return data;
		const match = /^\[(c|l|r|\*|)spacer[^\]]*\](.*)$/.exec(c.name);
		if (match == null) return data;
		data.name = match[2];
		if (match[1] === "*") {
			data.type = SpacerType.StarSpacer;
			data.name = match[2].repeat(50 / match[2].length);
		} else if (match[1] === "c") {
			data.type = SpacerType.CSpacer;
		} else if (match[1] === "l") {
			data.type = SpacerType.LSpacer;
		} else if (match[1] === "r") {
			data.type = SpacerType.RSpacer;
		}
		return data;
	}
</script>

<li class="container" class:hidden={!filterShow} class:collapsed>
	<div bind:this={div} on:mouseover={hover} on:mouseout={leave} class="hoverDummy">
		<div
			class="innerContainer"
			class:ownClient
			class:isSelected
			use:draggable={!!connection}
			on:svddrag={dragStart}
			on:svddrop={dragDrop}
			data-type="channel"
			data-key={$channel.id}>
			<button
				class="button collapseButton noBut"
				class:haschildren={$channels.length !== 0}
				class:spacer={spacerType !== SpacerType.None}
				on:click={() => (collapsed = !collapsed)}>
				<Icon name="chevron-right{collapsed ? '' : ' mdi-rotate-90'}" />
				<TsIcon type="channel" source={$channel} {connection} {server} />
			</button>
			<span
				class:spacerC={spacerType === SpacerType.CSpacer || spacerType === SpacerType.StarSpacer}
				class:spacerL={spacerType === SpacerType.LSpacer}
				class:spacerR={spacerType === SpacerType.RSpacer}
				class="nameBox"
				on:click={setChat}>
				{#if showId}
					[
					<FilterString filter={thisFilter} content={$channel.id.toString()} />]
				{/if}
				<FilterString filter={showId ? '' : thisFilter} content={displayName} />
			</span>
			{#if connection !== undefined}
				<span class="icons">
					<button class="button noBut" on:click={switchChannel}>
						<Icon name="shoe-print" />
					</button>
				</span>
			{/if}
		</div>
		{#if hovered}
			<div class="hover menu" style="top: {div.getBoundingClientRect().top}px;">
				<div class="corner" />
				{$channel.name}
			</div>
		{/if}
	</div>
	<ul class="menu-list">
		{#if connection !== undefined}
			{#each $clients as client (client.id)}
				<UiClient
					{connection}
					filter={childrenFilter}
					{client}
					bind:filterShow={client.filterShow} />
			{/each}
		{/if}
		{#each $channels as c (c.id)}
			<UiChannel
				{connection}
				{server}
				filter={childrenFilter}
				{filterStartFromRoot}
				channel={c}
				bind:filterShow={c.filterShow} />
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
			margin: 0 !important;
		}

		> :global(*) {
			overflow: hidden;
			text-overflow: ellipsis;
		}
	}

	.collapseButton {
		> :global(*:first-child) {
			opacity: 0;
		}
		&.spacer {
			> :global(*:last-child) {
				opacity: 0;
			}
		}
		&.spacer.haschildren {
			> :global(*:first-child) {
				opacity: 1;
			}
		}
		&.haschildren:hover {
			> :global(*:first-child) {
				opacity: 1;
			}
			> :global(*:last-child) {
				opacity: 0;
			}
		}
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

	@mixin spacer {
		text-overflow: clip;
	}

	.spacerL {
		@include spacer;
		text-align: start;
	}
	.spacerC {
		@include spacer;
		text-align: center;
	}
	.spacerR {
		@include spacer;
		text-align: end;
	}
</style>
