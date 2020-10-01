<script lang="typescript">
	import { Connection } from "../connection";
	import moment from "moment";
	import { datetimeDeserialize, LONG_DATETIME } from "../util";
	import Icon from "../ui/Icon.svelte";
	import PlatformIcon from "../ui/PlatformIcon.svelte";
	import ServerName from "../ui/ServerName.svelte";
	import TsIcon from "../ui/TsIcon.svelte";
	import StickyList from "../ui/StickyList.svelte";
	import StickySlot from "../ui/StickySlot.svelte";

	export let connection: Connection;
	const serverRaw = connection.book.server;
	$: server = $serverRaw;
	$: create_date =
		$server.created !== undefined ? datetimeDeserialize($server.created) : moment.unix(0);

	function disconnect() {
		connection.disconnect();
	}
</script>

<StickyList>
	<StickySlot>Info</StickySlot>
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
			<PlatformIcon platform={server.platform} />
			<div>{server.version}</div>
		</div>
		<div class="dataLine">
			<div>Created:</div>
			<div>{create_date.format(LONG_DATETIME)}</div>
		</div>
		<div class="dataLine">
			<div>Current Clients:</div>
			<div>{'?'} / {server.max_clients}</div>
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
