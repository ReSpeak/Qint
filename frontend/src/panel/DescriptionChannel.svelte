<script lang="typescript">
	import { Connection } from "../connection";
	import type { ChangePromise } from "../connection";
	import TsIcon from "../ui/TsIcon.svelte";
	import Icon from "../ui/Icon.svelte";
	import StickyList from "../ui/StickyList.svelte";
	import StickySlot from "../ui/StickySlot.svelte";
	import type { ChannelId } from "../ts";
	import { codecToName } from "../book";
	import RenderedText from "../ui/RenderedText.svelte";
	import StickyHeader from "./StickyHeader.svelte";
	import { app } from "../app";
	import { DescriptionMode } from "../transientSettings";

	export let connection: Connection;
	export let channelId: ChannelId;

	let statsOpen = false;
	let editMode = false;

	$: channelRaw = connection.book.getChannel(channelId)!;
	$: channel = $channelRaw;
	$: clients = channelRaw.clients;
	$: clientCount = $clients.length;
	let formatMaxClients: string | number = 0;
	$: {
		// TODO: calculate inheritance?
		if (channel.maxClients === "Inherited" || channel.maxClients === "Unlimited")
			formatMaxClients = channel.maxClients;
		else formatMaxClients = channel.maxClients?.Limited ?? "unknown";
	}

	// Load description
	$: descRequest = connection.sendChange({
		ChannelDescriptionRequest: {
			id: channelId,
		},
	});

	function editTopic(e: CustomEvent<{ text: string }>): ChangePromise {
		return connection.sendChange({
			ChannelEdit: {
				id: channelId,
				topic: e.detail.text,
			},
		});
	}

	function editDescription(e: CustomEvent<{ text: string }>): ChangePromise {
		return connection.sendChange({
			ChannelEdit: {
				id: channelId,
				description: e.detail.text,
			},
		});
	}
</script>

<StickyList>
	<StickySlot styled={false}>
		<StickyHeader title="Info">
			<button
				class="button is-small outline-button"
				class:active={editMode}
				on:click|stopPropagation={() => (editMode = !editMode)}>
				<Icon name="pencil" />
				<span>Edit</span>
			</button>
		</StickyHeader>
	</StickySlot>
	<div class="descGroup">
		<div class="dataLine headLine">
			<TsIcon type="channel" source={channel} {connection} />
			<div>{channel.name}</div>
			<div style="flex: 1;" />
			<span class="tag is-primary is-rounded">{channel.channelType}</span>
		</div>
		<div class="dataLine">
			<div>Topic:</div>
			<div>
				<RenderedText
					text={channel.topicRendered ?? ''}
					raw={channel.topic ?? undefined}
					editable={true}
					on:edited={editTopic} />
			</div>
		</div>
		<div class="dataLine">
			<div>Codec:</div>
			<div>{channel.codec !== null ? codecToName(channel.codec) : 'unknown'}</div>
			<div style="margin: 0 0.3em">@</div>
			<div>{channel.codecQuality}</div>
		</div>
		<div class="dataLine">
			<div>Current Clients:</div>
			<div>{clientCount} / {formatMaxClients}</div>
		</div>
	</div>
	<hr />
	<div class="description">
		{#await descRequest then descRequestResult}
			<!-- Todo check properly -->
			{#if descRequestResult !== undefined}
				<span style="color: red;">Missing permission</span>
			{:else}
				<RenderedText
					text={channel.descriptionRendered ?? ''}
					raw={channel.description ?? undefined}
					editable={true}
					on:edited={editDescription} />
			{/if}
		{/await}
	</div>
	<StickySlot>Settings</StickySlot>
	klik here for party
	<StickySlot on:click={() => (statsOpen = true)}>
		<button class="button iconButton" on:click|stopPropagation={() => (statsOpen = !statsOpen)}>
			<Icon name="chevron-right{statsOpen ? ' mdi-rotate-90' : ''}" />
		</button>
		<span>Stats</span>
	</StickySlot>
	{#if statsOpen}Test{/if}
</StickyList>

<style>
	.description {
		margin: 1em;
	}
</style>
