<script lang="ts">
	import { afterUpdate, onMount } from "svelte";
	import type { Writable } from "svelte/store";
	import { get } from "svelte/store";
	import Icon from "../ui/icon/Icon.svelte";
	import TsIcon from "../ui/icon/TsIcon.svelte";
	import HighlightString from "../ui/specialized/HighlightString.svelte";
	import type { ResultDetails } from "../backend/ws";
	import { Channel } from "../book";
	import type { ITreeNode } from "../book";
	import UiClient from "./ClientWrap.svelte";
	import UiChannel from "./ChannelWrap.svelte";
	import { Connection, DDConnection } from "../connection";
	import { draggable, DragData, MouseButton } from "../ui/util/draggable";
	import { findParent, assert, flash, focus, render_updates } from "../util";
	import { SpacerType } from "./tree";
	import { app, NodeSelection } from "../app";
	import { ChannelType, TsError } from "../book_events";
	import HoverMenu from "./HoverMenu.svelte";
	import { DelayedHover } from "./delayedHover";
	import { DescriptionMode } from "../transientSettings";
	import HoverContainer from "./HoverContainer.svelte";
	import ChangeResult from "../ui/specialized/ChangeResult.svelte";
	import debug from "debug";
	const log = debug("UICHANNEL");

	if (render_updates) afterUpdate(() => flash(div));

	export let connection: Connection | undefined = undefined;
	export let server: string | undefined = undefined;
	export let filter: string;
	export let filterShow: boolean = true;
	export let filterStartFromRoot: boolean;
	export let channel: Channel;

	let collapsed = false;
	let showId = false;
	let thisFilter = "";
	let childrenFilter = "";
	let hover: DelayedHover;
	let hovered: Writable<boolean>;
	let askPassword: string | undefined;
	let error: ResultDetails | undefined;

	$: isSelected = $channel.isSelected;
	$: channels = channel.channels;
	$: clients = channel.clients;
	$: filterShow = applyFilter(filter, filterStartFromRoot, $channel, $channels, $clients);
	// Update if a client moves in or out
	$: ownClient = updateOwnClient($clients);
	$: chat = channel.chat;

	let spacerType: SpacerType;
	let displayName: string;
	$: {
		const chanData = getDisplayName($channel);
		spacerType = chanData.type;
		displayName = chanData.name;
	}

	let div: HTMLElement;

	function updateOwnClient(_children: ITreeNode[]) {
		if (connection === undefined) return false;
		const client = get(connection.book.ownClient);
		if (client === undefined) return false;
		return client.channel === channel.id;
	}

	function applyFilter(
		filter: string,
		filterStartFromRoot: boolean,
		channel: Channel,
		channels: ITreeNode[],
		clients: ITreeNode[]
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
			const matches = channel.id.toString().includes(filter.substring(1));
			if (!showId) showId = true;
			if (childrenFilter !== filter) childrenFilter = filter;
			if (thisFilter !== filter.substring(1)) thisFilter = filter.substring(1);
			return matches || children.some((c) => c.filterShow);
		} else {
			if (showId) showId = false;
			const index = filter.indexOf("/");
			const newThisFilter = index === -1 ? filter : filter.substring(0, index);
			if (thisFilter !== newThisFilter) thisFilter = newThisFilter;
			const matches = channel.name.toLowerCase().includes(thisFilter.toLowerCase());
			if (filterStartFromRoot) {
				let newChildrenFilter = "";
				if (index !== -1) newChildrenFilter = filter.substring(index + 1);
				if (childrenFilter !== newChildrenFilter) childrenFilter = newChildrenFilter;
				return matches && (childrenFilter === "" || children.some((c) => c.filterShow));
			}
			if (childrenFilter !== filter) childrenFilter = filter;
			return matches || children.some((c) => c.filterShow);
		}
	}

	async function switchChannel(ev?: MouseEvent) {
		if (connection === undefined) return;
		if (ev !== undefined && ev.button !== MouseButton.Main) return;
		const res = await connection.switchChannel(channel, askPassword);
		if (res !== undefined) {
			console.log("Failed to switch channel", res);
			error = res;
			if (res.tsResult === TsError.ChannelInvalidPassword) {
				if (askPassword === undefined) askPassword = "";
			}
		}
	}

	function switchWithPassword() {
		error = undefined;
		switchChannel();
		askPassword = undefined;
		hovered.set(false);
	}

	function closeAskPassword() {
		error = undefined;
		askPassword = undefined;
		hovered.set(false);
	}

	function preventScrollClick(ev: MouseEvent): any {
		if (ev.button === MouseButton.Auxiliary) {
			ev.preventDefault();
			return false;
		}
	}

	function setChat(ev: MouseEvent) {
		if (connection === undefined) return;
		if (ev.button === MouseButton.Main) {
			if (ev.ctrlKey) {
				app.toggleSelection(new NodeSelection(connection, channel));
			} else if (ev.shiftKey) {
				app.expandSelection(new NodeSelection(connection, channel));
			} else {
				app.setDescriptionMode(
					new NodeSelection(connection, channel),
					DescriptionMode.Info
				);
			}
		} else if (ev.button === MouseButton.Auxiliary) {
			app.setDescriptionMode(new NodeSelection(connection, channel), DescriptionMode.Files);
		}
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
		const dropTarget = hoverOpt.find(
			(x) => x.dataset.type === "channel" || x.dataset.type === "client"
		);
		if (dropTarget !== undefined && connection !== undefined) {
			const rect = dropTarget.getBoundingClientRect();
			const clickY = ev.detail.mouseDrop.clientY - rect.top;
			let clickPerc = clickY / (rect.bottom - rect.top);
			let target: Channel | undefined;
			if (dropTarget.dataset.type === "channel") {
				target = connection.book.getChannel(dropTarget.dataset.key!);
			} else {
				const client = connection.book.getClient(dropTarget.dataset.key!);
				if (client !== undefined) target = connection.book.getChannel(client.channel);
				clickPerc = 0.5;
			}
			if (target === undefined) {
				log("Target channel for drag'n'drop not found");
				return;
			}

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
				connection.moveChannel(channel.id, target.id, "0");
			} else {
				// Dropped BOTTOM
				if (get(target.channels).length > 0) {
					// Case C: Channel HAS child
					//      => Same as middle
					connection.moveChannel(channel.id, target.id, "0");
				} else {
					// Case D: Channel NO child
					//      => Place below target, parent same as target, order is target
					connection.moveChannel(channel.id, target.parent, target.id);
				}
			}

			//console.log("Would drop", channel.id, "to", dropTarget.dataset.key, "at", clickPerc);
		}
	}

	function getDisplayName(c: Channel) {
		// TODO consider special names [*spacer] --- ... -.- ___ -..
		const data = { type: SpacerType.None, name: c.name };
		if (c.parent !== "0" || c.channelType !== ChannelType.Permanent) return data;
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

	onMount(() => {
		hover = new DelayedHover(div, [div]);
		hovered = hover.hovered;

		return () => hover.unregister();
	});
</script>

<li class="container" class:hidden={!filterShow} class:collapsed>
	<div bind:this={div} tabindex="0" class="hoverDummy">
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
				class:haschildren={$channels.length !== 0 || $clients.length !== 0}
				class:spacer={spacerType !== SpacerType.None}
				on:click={() => (collapsed = !collapsed)}>
				<Icon name="chevron-right{collapsed ? '' : ' mdi-rotate-90'}" />
				<TsIcon
					type="channel"
					source={$channel}
					connection={new DDConnection(connection, server)} />
			</button>
			<span
				class:spacerC={spacerType === SpacerType.CSpacer ||
					spacerType === SpacerType.StarSpacer}
				class:spacerL={spacerType === SpacerType.LSpacer}
				class:spacerR={spacerType === SpacerType.RSpacer}
				class="nameBox"
				on:mousedown={preventScrollClick}
				on:click={setChat}
				on:auxclick={setChat}
				on:dblclick={switchChannel}>
				{#if showId}
					[<HighlightString filter={thisFilter} content={$channel.id.toString()} />]
				{/if}
				<HighlightString filter={showId ? "" : thisFilter} content={displayName} />
			</span>
			<span class="icons">
				{#if $chat.unreadCount > 0}
					<span class="unreadCount" title={$chat.unreadCount.toString()}>
						{#if $chat.unreadCount >= 100}99+{:else}{$chat.unreadCount}{/if}
					</span>
				{/if}
			</span>
		</div>
		{#if askPassword !== undefined}
			<div
				class="askPasswordHoverContainer"
				on:keydown={(e) => {
					if (e.key === "Escape") closeAskPassword();
				}}
				tabindex="0">
				<HoverContainer {div} closeButton={true} on:close={closeAskPassword}>
					{#if error !== undefined}
						<ChangeResult result={error} />
					{/if}
					<form class="field has-addons" on:submit|preventDefault={switchWithPassword}>
						<div class="control">
							<input
								bind:value={askPassword}
								in:focus|local
								name="password"
								class="input"
								type="password"
								title="Password"
								placeholder="Password" />
						</div>
						<div class="control">
							<button class="button" name="switch" type="submit">
								<Icon name="check" />
							</button>
						</div>
					</form>
				</HoverContainer>
			</div>
		{:else if error !== undefined}
			<div class="errorHoverContainer">
				<HoverContainer
					{div}
					closeButton={true}
					on:close={() => {
						error = undefined;
						hovered.set(false);
					}}>
					<ChangeResult result={error} />
				</HoverContainer>
			</div>
		{:else if connection !== undefined && $hovered}
			<HoverMenu {div} selected={new NodeSelection(connection, channel)} />
		{/if}
	</div>
	<ul class="menu-list">
		{#if connection !== undefined}
			{#each $clients as client (client.id)}
				<UiClient
					{connection}
					filter={childrenFilter}
					{client}
					{channel}
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

	.collapseButton {
		position: relative;
		// This should match the icon w/h
		width: 1.5em;
		height: 1.5em;

		> :global(*) {
			transition: all 0.1s;
			position: absolute;
			margin: 0 !important;

			overflow: hidden;
			text-overflow: ellipsis;
		}

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
		margin: 0 0 0 0.9em;
		padding-left: 0.1em;
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

	.errorHoverContainer :global(.hover),
	.errorHoverContainer :global(.hover .corner) {
		background-color: mix($background, $danger, 50) !important;
	}

	.errorHoverContainer :global(.hover) {
		align-items: center;
		margin-top: 1em;
	}

	.errorHoverContainer :global(.hover .corner) {
		top: 1em !important;
	}

	.errorHoverContainer :global(.hover),
	.askPasswordHoverContainer :global(.hover) {
		display: grid !important;
		grid-template-columns: 1fr auto;
	}

	.askPasswordHoverContainer :global(.hover) {
		:global(.closeButton) {
			grid-row: 1;
			grid-column: 2;
		}
		:global(.changeResult) {
			grid-row: 1;
			grid-column: 1;
		}
		:global(:last-child) {
			grid-column: 1 / 3;
		}
	}
</style>
