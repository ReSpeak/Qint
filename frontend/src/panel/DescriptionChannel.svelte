<script lang="typescript">
	import { Connection } from "../connection";
	import TsIcon from "../ui/TsIcon.svelte";
	import StickyList from "../ui/StickyList.svelte";
	import StickySlot from "../ui/StickySlot.svelte";
	import { codecToName } from "../ts";

	export let connection: Connection;
	export let channelId: number;

	$: channelRaw = connection.book.getChannel(channelId)!;
	$: channel = $channelRaw;
	$: clients = channelRaw.clients;
	$: clientCount = $clients.length;
	let formatMaxClients: string | number = 0;
	$: {
		// TODO: calculate inheritance?
		if (channel.max_clients === "Inherited" || channel.max_clients === "Unlimited")
			formatMaxClients = channel.max_clients;
		else
			formatMaxClients = channel.max_clients?.Limited ?? "unknown";
	}
</script>

<StickyList>
	<StickySlot>Info</StickySlot>
	<div class="descGroup">
		<div class="dataLine headLine">
			<TsIcon type="channel" source={channel} {connection} />
			<div>{channel.name}</div>
			<div style="flex: 1;" />
			<span class="tag is-primary is-rounded">{channel.channel_type}</span>
		</div>
		<div class="dataLine">
			<div>Topic:</div>
			<div>{channel.topic}</div>
		</div>
		<div class="dataLine">
			<div>Codec:</div>
			<div>{channel.codec !== null ? codecToName(channel.codec) : "unknown"}</div>
			<div style="margin: 0 0.3em">@</div>
			<div>{channel.codec_quality}</div>
		</div>
		<div class="dataLine">
			<div>Current Clients:</div>
			<div>{clientCount} / {formatMaxClients}</div>
		</div>
	</div>
	<StickySlot>Settings</StickySlot>
	klik here for party
</StickyList>

<style>
</style>
