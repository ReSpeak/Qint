<script lang="typescript">
	import { Connection } from "../connection";
	import TsIcon from "../ui/TsIcon.svelte";
	import StickyList from "../ui/StickyList.svelte";
	import StickySlot from "../ui/StickySlot.svelte";
	import type { ChannelId } from "../ts";
	import { codecToName } from "../book";
	import RenderedText from "../ui/RenderedText.svelte";

	export let connection: Connection;
	export let channelId: ChannelId;

	$: channelRaw = connection.book.getChannel(channelId)!;
	$: channel = $channelRaw;
	$: clients = channelRaw.clients;
	$: clientCount = $clients.length;
	let formatMaxClients: string | number = 0;
	$: {
		// TODO: calculate inheritance?
		if (channel.maxClients === "Inherited" || channel.maxClients === "Unlimited")
			formatMaxClients = channel.maxClients;
		else
			formatMaxClients = channel.maxClients?.Limited ?? "unknown";
	}

	// Load description
	$: connection.sendMessage({ Change: {
		ChannelDescriptionRequest: {
			id: channelId,
		},
	}});
</script>

<StickyList>
	<StickySlot>Info</StickySlot>
	<div class="descGroup">
		<div class="dataLine headLine">
			<TsIcon type="channel" source={channel} {connection} />
			<div>{channel.name}</div>
			<div style="flex: 1;" />
			<span class="tag is-primary is-rounded">{channel.channelType}</span>
		</div>
		<div class="dataLine">
			<div>Topic:</div>
			<div>{channel.topic}</div>
		</div>
		<div class="dataLine">
			<div>Codec:</div>
			<div>{channel.codec !== null ? codecToName(channel.codec) : "unknown"}</div>
			<div style="margin: 0 0.3em">@</div>
			<div>{channel.codecQuality}</div>
		</div>
		<div class="dataLine">
			<div>Current Clients:</div>
			<div>{clientCount} / {formatMaxClients}</div>
		</div>
	</div>
	<div class="description">
		{#if channel.descriptionRendered}
			<RenderedText text={channel.descriptionRendered} raw={channel.description ?? undefined} />
		{/if}
	</div>
	<StickySlot>Settings</StickySlot>
	klik here for party
</StickyList>

<style>
	.description {
		margin: 1em;
	}
</style>
