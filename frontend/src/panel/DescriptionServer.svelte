<script lang="typescript">
	import { Connection } from "../connection";
	import moment from "moment";
	import { LONG_DATETIME } from "../util";
	import Icon from "../ui/Icon.svelte";
	import PlatformIcon from "../ui/PlatformIcon.svelte";
	import ServerName from "../ui/ServerName.svelte";
	import TsIcon from "../ui/TsIcon.svelte";
	import StickyList from "../ui/StickyList.svelte";
	import StickySlot from "../ui/StickySlot.svelte";
	import StickyHeader from "./StickyHeader.svelte";
	import RenderedText from "../ui/RenderedText.svelte";

	export let connection: Connection;
	const serverRaw = connection.book.server;
	$: server = $serverRaw;
	$: create_date = server.created !== undefined ? server.created : moment.unix(0);

	$: connection.sendMessage({
		Change: {
			change: {
				ServerVariablesRequest: {},
			},
		},
	});

	function disconnect() {
		connection.disconnect();
	}

	function editHostmessage(e: CustomEvent<{ text: string }>) {
		connection.sendMessage({
			Change: {
				change: {
					ServerEdit: {
						hostmessage: e.detail.text,
					},
				},
			},
		});
	}

	function editWelcomeMessage(e: CustomEvent<{ text: string }>) {
		connection.sendMessage({
			Change: {
				change: {
					ServerEdit: {
						welcomeMessage: e.detail.text,
					},
				},
			},
		});
	}
</script>

<StickyList>
	<StickySlot styled={false}>
		<StickyHeader title="Info" />
	</StickySlot>
	<div class="descGroup">
		<div class="dataLine headLine">
			<TsIcon type="server" source={server} {connection} />
			<ServerName {connection} />
		</div>
		<div class="dataLine">
			<div>IPs:</div>
			<div>{server.ips?.join(', ') ?? ''}</div>
			{#if server.nickname}
				<span style="margin-left:1em;">(Nickname: </span>
				<code class="nick">{server.nickname}</code>
				<span>)</span>
			{/if}
		</div>
		<div class="dataLine">
			<div>License:</div>
			<div>{server.license}</div>
		</div>
		<div class="dataLine">
			<div>Version:</div>
			<PlatformIcon platform={server.platform} version={server.version} />
		</div>
		<div class="dataLine">
			<div>Host message:</div>
			<RenderedText text={server.hostmessageRendered ?? ''} />
		</div>
		<div class="dataLine">
			<div>Welcome message:</div>
			<RenderedText text={server.welcomeMessageRendered ?? ''} />
		</div>
		<div class="dataLine">
			<div>Created:</div>
			<div>{create_date.format(LONG_DATETIME)}</div>
		</div>
		<div class="dataLine">
			<div>Current Clients:</div>
			<div>{'?'} / {server.maxClients}</div>
		</div>
	</div>
	<StickySlot>Actions</StickySlot>
	<div class="descGroup">
		<p class="buttons">
			<button class="button is-small is-warning" on:click={disconnect}>
				<Icon name="" />
				<span>Disconnect</span>
			</button>
		</p>
	</div>
</StickyList>

<style lang="scss">
	.nick {
		padding: 0 0.3em;
		margin: 0 0.3em;
		border-radius: 5px;
	}
</style>
