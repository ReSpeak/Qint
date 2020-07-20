<script lang="typescript">
	import { Connection } from "../connection";
	import { Writable } from "svelte/store";
	import { Channel, Client, ITreeNode} from "../tree/book";
	import { Moment } from "moment";
	import ChannelIcon from "../ui/ChannelIcon.svelte";
	import { Codec, codecToName } from "../structs/ts";

	export let connection!: Connection;
	export let channelId!: number;

	let channel: Channel;
	$: channel = connection.book.getChannel(channelId)!;
	let children: Writable<ITreeNode[]>;
	$: children = channel.children;
	let clientCount: number;
	$: clientCount = $children.filter((c: ITreeNode) => c instanceof Client).length;
	let formatMaxClients: string | number;
	$: {
		// TODO: calculate inheritance?
		if (channel.max_clients === "Inherited" || channel.max_clients === "Unlimited")
			formatMaxClients = channel.max_clients;
		else
			formatMaxClients = channel.max_clients.Limited;
	}
</script>

<div class="descGroup">
	<div class="dataLine headLine">
		<ChannelIcon {channel} {connection} />
		<div>{channel.name}</div>
		<div style="flex: 1;" ></div>
		<span class="tag is-primary is-rounded">{channel.channel_type}</span>
	</div>
	<div class="dataLine">
		<div>Topic:</div>
		<div>{channel.topic}</div>
	</div>
	<div class="dataLine">
		<div>Codec:</div>
		<div>{codecToName(channel.codec)}</div>
		<div style="margin: 0 0.3em">@</div>
		<div>{channel.codec_quality}</div>
	</div>
	<div class="dataLine">
		<div>Current Clients:</div>
		<div>{clientCount} / {formatMaxClients}</div>
	</div>
</div>

<style>
</style>
