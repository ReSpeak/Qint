<script lang="typescript">
	import type { Readable } from "svelte/store";
	import { writable } from "svelte/store";
	import TsIcon from "../ui/TsIcon.svelte";
	import ServerGroupIcon from "../ui/ServerGroupIcon.svelte";
	import FilterString from "../ui/FilterString.svelte";
	import Icon from "../ui/Icon.svelte";
	import SimpleDiagram from "../ui/UiSimpleDiagram.svelte";
	import { Connection } from "../connection";
	import { Channel, Client } from "../book";
	import { draggable, DragData } from "../ui/draggable";
	import { findParent, flash, LOUDNESS_HISTORY, LOUDNESS_MAX, LOUDNESS_MIN, on, render_updates } from "../util";
	import { afterUpdate, onMount } from "svelte";
	import { app, NodeSelection } from "../app";
	import type { ServerGroupId } from "../ts";
	import HoverMenu from "./HoverMenu.svelte";
	import { DelayedHover } from "./delayedHover";

	if (render_updates) afterUpdate(() => flash(div));

	export let connection: Connection;
	export let client: Client;
	export let filter: string;
	export let filterShow: boolean = true;
	// Channel where this client is in
	export let channel: Channel | undefined = undefined;
	let hover: DelayedHover;
	let hovered: Readable<boolean>;
	let showId = false;
	let thisFilter = "";
	const serverGroups = connection.book.serverGroups;

	$: isSelected = $client.isSelected;
	$: chat = client.chat;
	$: filterShow = applyFilter(filter, $client);
	$: clientProperties = getClientProperties($client, channel !== undefined ? $channel : undefined);
	let ownClient = client.id === connection.book.ownClientId;
	let div: HTMLElement;
	let loudnessDiagram: SimpleDiagram;
	let loudness = writable(LOUDNESS_MIN);

	let sortedServerGroups: ServerGroupId[];
	$: {
		// Also depend on server groups
		on($serverGroups);
		sortedServerGroups = connection.book.sortServerGroupIds($client.serverGroups);
	}

	function getClientProperties(client: Client, channel: Channel | undefined): [string, string, string] | undefined {
		const properties: [boolean, string, string, string][] = [
			[client.awayMessage !== null, "sleep", "color: rgb(70,180,255)", client.awayMessage === "" ? "Away" : client.awayMessage ?? ""],
			[!client.outputHardwareEnabled, "microphone-off", "color: red;", "Speaker disabled"],
			[client.outputMuted, "volume-off", "color: red;", "Deaf"],
			[channel !== undefined && channel.neededTalkPower !== null && client.talkPower < channel.neededTalkPower,
				"microphone-off", "color: gray;", "Not enough talk power"],
			[!client.inputHardwareEnabled, "microphone-off", "color: red;", "Microphone disabled"],
			[client.inputMuted, "microphone-off", "color: red;", "Muted"],
		];
		let resEntry: [string, string] | undefined;
		let resDescription = "";
		for (const p of properties) {
			if (p[0]) {
				if (resEntry === undefined) {
					resEntry = [p[1], p[2]];
					resDescription = p[3];
				} else {
					resDescription += " | ";
					resDescription += p[3];
				}
			}
		}

		if (resEntry === undefined)
			return undefined;
		return [resEntry[0], resEntry[1], resDescription];
	}

	function setChat() {
		app.select(connection, client);
	}

	function applyFilter(filter: string, client: Client) {
		if (filter === "") {
			if (showId) showId = false;
			if (thisFilter !== filter) thisFilter = filter;
			return true;
		}
		const filterById = filter[0] === "/";
		if (filterById) {
			if (!showId) showId = true;
			if (thisFilter !== filter.substr(1)) thisFilter = filter.substr(1);
			return client.id.toString().includes(filter.substr(1));
		} else {
			if (showId) showId = false;
			if (thisFilter !== filter) thisFilter = filter;
			return client.name.toLowerCase().includes(filter.toLowerCase());
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
		const hoverOpt: HTMLElement[] = [...ev.detail.customData.querySelectorAll(":hover")];
		const dropTarget = hoverOpt.reverse().find((x) => x.dataset.type === "channel");
		console.log(hoverOpt, dropTarget);
		if (dropTarget !== undefined) {
			console.log("Would drop to", dropTarget.dataset.key);
			connection.moveClient(client.id, dropTarget.dataset.key!);
		}
	}

	onMount(() => {
		hover = new DelayedHover(div, [div]);
		hovered = hover.hovered;

		connection.loudnesses[client.id] = loudness;
		loudness.subscribe(l => {
			if (loudnessDiagram !== undefined)
				loudnessDiagram.addValue(l);
		});

		return () => {
			hover.unregister();

			delete connection.loudnesses[client.id];
		};
	});
</script>

<li class="container" class:hidden={!filterShow}>
	<div bind:this={div} tabindex="0" class="hoverDummy">
		<div
			class:ownClient
			class:isSelected
			class="innerContainer"
			on:click={setChat}
			use:draggable={!!connection}
			on:svddrag={dragStart}
			on:svddrop={dragDrop}
			data-type="client"
			data-key={$client.id}>
			<!--<div class:talking={$client.talking !== TalkState.Off} class="talkWave" />-->
			<div class="talkWave talking">
				<SimpleDiagram bind:this={loudnessDiagram}
					style="width: 100%; height: 100%"
					min={LOUDNESS_MIN}
					max={LOUDNESS_MAX}
					count={LOUDNESS_HISTORY}
				/>
			</div>
			<TsIcon type="client" source={$client} {connection} />
			<span class="nameBox" style="color:{$client.color};">
				{#if showId}
					[<FilterString filter={thisFilter} content={$client.id.toString()} />]
				{/if}
				<FilterString filter={showId ? '' : thisFilter} content={$client.name} />
			</span>
			<span class="icons">
				{#if clientProperties !== undefined}
					<Icon name={clientProperties[0]} style={clientProperties[1]} title={clientProperties[2]} />
				{/if}
				{#each sortedServerGroups as grp (grp)}
					<ServerGroupIcon id={grp} {connection} />
				{/each}
				{#if $client.clientType !== "Normal"}
					{#if $client.clientType.Query.admin}
						<Icon name="robot-outline" title="Admin Serverquery" />
					{:else}
						<Icon name="robot-outline" title="Serverquery" />
					{/if}
				{/if}
				{#if $chat.unreadCount > 0}
					<span class="unreadCount" title={$chat.unreadCount.toString()}>
						{#if $chat.unreadCount >= 100}
							99+
						{:else}
							{$chat.unreadCount}
						{/if}
					</span>
				{/if}
			</span>
		</div>
		{#if $hovered}
			<HoverMenu {div} selected={new NodeSelection(connection, client)} />
		{/if}
	</div>
</li>

<style lang="scss">
	@import "./tree";

	.talkWave {
		transition: opacity 0.2s ease-in-out, height 0.2s ease-in-out;
		position: absolute;
		// top: 0;
		right: 0;
		bottom: 0;
		left: 0;

		//background-image: url("/talking.svg");
		background-size: 100% auto;
		//-webkit-mask-image: radial-gradient(rgba(0, 0, 0, 1), rgba(0, 0, 0, 0));
		//mask-image: radial-gradient(rgba(0, 0, 0, 1), rgba(0, 0, 0, 0));

		opacity: 0;
		height: 50%;
		z-index: -1;
	}

	.talkWave.talking {
		opacity: 1;
		height: 100%;
	}
</style>
