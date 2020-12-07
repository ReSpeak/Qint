<script lang="typescript">
	import { get } from "svelte/store";
	import { Connection } from "../connection";
	import type { ServerGroupId } from "../ts";
	import moment from "moment";
	import type { Duration } from "moment";
	import Icon from "../ui/Icon.svelte";
	import PlatformIcon from "../ui/PlatformIcon.svelte";
	import ServerGroupIcon from "../ui/ServerGroupIcon.svelte";
	import TsIcon from "../ui/TsIcon.svelte";
	import ClientName from "../ui/ClientName.svelte";
	import StickyList from "../ui/StickyList.svelte";
	import StickySlot from "../ui/StickySlot.svelte";
	import StickyHeader from "./StickyHeader.svelte";
	import ClientVolume from "../ui/ClientVolume.svelte";
	import { getClientAvatarPath } from "../ui/clientIcon";
	import { Reason } from "../book_events";
	import { onMount } from "svelte";
	import { NARROW_NO_BREAK_SPACE, on } from "../util";
	import { Client, ServerGroup } from "../book";
	import BModal from "../ui/BModal.svelte";
	import { tick } from "svelte";
	import BChart from "../ui/BChart.svelte";
	import { app } from "../app";

	export let connection: Connection;
	export let client: Client;

	const CHART_ENTRY_COUNT = 61;

	let statsOpen = false;
	let pokeModalVisible = false;
	let pokeInput: HTMLElement | undefined;
	let pokeMessage: string = "";
	let developMode = app.transientSettings.ui._developMode;
	let chart: BChart | null = null;

	const serverGroups = connection.book.serverGroups;
	$: avatarPath = getClientAvatarPath($client, connection);
	$: ownClient = client.id === connection.book.ownClientId;
	$: {
		if ($client.optionalData == null) getOptionalData();
		if ($client.connectionData == null) getConnectionData();
	}
	$: on(client, clearChart());

	let chartConfig: Chart.ChartConfiguration = {
		type: "line",
		data: {
			datasets: [{
				label: "Ping",
				data: [],
				backgroundColor: "#87A23600",
				borderColor: "#87A236FF",
				pointRadius: 1
			}, {
				label: "Packet loss to Server",
				data: [],
				backgroundColor: "#9F354800",
				borderColor: "#9F3548FF",
				pointRadius: 1
			}, {
				label: "Packet loss from Server",
				data: [],
				backgroundColor: "#512C7300",
				borderColor: "#512C73FF",
				pointRadius: 1
			}]
		},
		options: {
			scales: {
				xAxes: [{
					type: "time",
					time: {
						unit: "second",
						displayFormats: {
							second: "X"
						},
						stepSize: 5
					},
					ticks: {
						callback: function(value) {
							let seconds = moment().diff(moment(value, "X"), "seconds");
							if (seconds < 1) return "0";
							return `${seconds}s`;
						}
					}
				}],
				yAxes: [{
					ticks: {
						beginAtZero: true,
						suggestedMax: 100,
						maxTicksLimit: 5,
						callback: function(value) {
							return `${value}ms`;
						}
					}
				}, {
					ticks: {
						beginAtZero: true,
						suggestedMax: 5,
						maxTicksLimit: 5,
						callback: function(value) {
							return `${Number(value)}%`;
						}
					}
				}]
			}
		}
	};

	interface ExtendedGroup {
		isMember: boolean;
		inner: ServerGroup;
	}

	let groups: ExtendedGroup[];
	$: {
		groups = [];
		$serverGroups.forEach((group, id) => {
			const g = get(group);
			if (g.groupType === "Regular") {
				groups.push({
					isMember: $client.serverGroups.includes(id),
					inner: g,
				});
			}
		});
		// Sort alphabetically
		groups.sort((a, b) => {
			return a.inner.cmp(b.inner);
		});
	}

	function clearChartData() {
		console.log("clear");
		chartConfig.data?.datasets?.forEach(ds => ds.data = []);
	}

	function createDataPoint(value: number | undefined): Chart.ChartPoint {
		return {
			x: moment(),
			y: value,
		};
	}

	function packetLossToPercent(loss01: number | null | undefined): number | undefined {
		if (!loss01) return undefined;
		return loss01 * 100;
	}

	function addChartValue(data: (number | number[] | null | undefined)[] | Chart.ChartPoint[] | undefined, newValue: number | undefined) {
		if (!data) return;

		data.push(createDataPoint(newValue));

		while (data.length > CHART_ENTRY_COUNT) {
			data.shift();
		}
	}

	function chartRefresh() {
		addChartValue(chartConfig.data?.datasets![0].data, client.connectionData ? client.connectionData.ping?.asMilliseconds() : undefined);
		addChartValue(chartConfig.data?.datasets![1].data, packetLossToPercent(client.connectionData?.clientToServerPacketlossTotal));
		addChartValue(chartConfig.data?.datasets![2].data, packetLossToPercent(client.connectionData?.serverToClientPacketlossTotal));
		chart?.updateChart?.();
	}

	function clearChart() {
		chartConfig.data?.datasets?.filter(dataset => dataset.data).forEach(dataset => {
			dataset.data = [];
			for (let i = CHART_ENTRY_COUNT; i > 0; i--) {
				let entry = {
					x: moment().subtract(i, "second"),
					y: undefined
				};
				dataset.data.push(entry);
			}
		});
		chart?.updateChart?.();
	}

	function changeServerGroup(e: Event, group: ServerGroupId, isMember: boolean) {
		if (e.target instanceof HTMLInputElement) e.target.disabled = true;

		if (isMember) {
			connection.sendMessage({
				Change: {
					change: {
						ClientAddServerGroup: {
							id: client.id,
							serverGroup: group,
						},
					},
				},
			});
		} else {
			connection.sendMessage({
				Change: {
					change: {
						ClientRemoveServerGroup: {
							id: client.id,
							serverGroup: group,
						},
					},
				},
			});
		}
	}

	function kickFromChannel() {
		connection.sendMessage({
			Change: {
				change: {
					ClientKick: {
						id: client.id,
						reason: Reason.KickChannel,
					},
				},
			},
		});
	}

	function kickFromServer() {
		connection.sendMessage({
			Change: {
				change: {
					ClientKick: {
						id: client.id,
						reason: Reason.KickServer,
					},
				},
			},
		});
	}

	async function updateClientInfo() {
		chartRefresh();
		await connection
			.sendChange({
				ClientConnectionInfoRequest: {
					id: client.id,
				},
			})
			.catch((reason) => {
				console.error("Client info update failed: ", reason);
			});
	}

	async function getOptionalData() {
		await connection
			.sendChange({
				ClientVariablesRequest: {
					id: client.id,
				},
			})
			.catch((reason) => {
				console.error("ClientVariablesRequest failed: ", reason);
			});
	}

	async function getConnectionData() {
		await connection
			.sendChange({
				ClientConnectionInfoRequest: {
					id: client.id,
				},
			})
			.catch((reason) => {
				console.error("ClientConnectionInfoRequest failed: ", reason);
			});
	}

	async function onPokeClick() {
		pokeModalVisible = true;
		await tick();
		if (pokeInput !== undefined) pokeInput.focus();
	}

	function onPokeSend() {
		connection.pokeClient(client.id, pokeMessage);
		pokeModalVisible = false;
		pokeMessage = "";
	}

	function countryCodeToEmojis(countryCode: string): string {
		return [...countryCode]
			.map((char) => String.fromCodePoint(char.charCodeAt(0) + 127397))
			.join("");
	}

	function formatClientPing(client: Client): string {
		if (client.connectionData === null || client.connectionData.ping === null || client.connectionData.pingDeviation === null) return "";

		return `(${Math.round(client.connectionData.ping.asMilliseconds() * 10) / 10} ± ${
			Math.round(client.connectionData.pingDeviation.asMilliseconds() * 10) / 10
		})${NARROW_NO_BREAK_SPACE}ms`;
	}

	function formatPacketLoss(...losses: (number | null | undefined)[]) {
		const filteredLosses = losses.flatMap(a => a !== null && a !== undefined ? [a] : []);
		if (filteredLosses.length === 0) {
			return "unknown";
		} else {
			const reduced = filteredLosses.reduce((a, b) => a + b);
			const totalLoss = Math.round(reduced * 10) / 10;
			return `${totalLoss}${NARROW_NO_BREAK_SPACE}%`;
		}
	}

	function formatPacketCount(...packetCounts: (string | null | undefined)[]) {
		let totalCount =
			Math.round(
				packetCounts.map((x) => (x ? parseInt(x) : 0)).reduce((a, b) => a + b) / 100
			) / 10;
		return `${totalCount}${NARROW_NO_BREAK_SPACE}k`;
	}

	function formatAgo(duration: Duration | null | undefined, ago: boolean): string {
		if (!duration) return "";
		if (ago) return moment.duration(-duration.asSeconds(), "seconds").humanize(true);
		else return moment.duration(duration.asSeconds(), "seconds").humanize();
	}

	function formatDuration(duration: Duration | null | undefined): string {
		if (!duration) return "";
		const asSec = Math.floor(duration.asSeconds());
		if (asSec <= 60) return `${asSec}s`;
		const asMin = Math.floor(duration.asMinutes());
		const floorSec = Math.floor(duration.seconds());
		if (asMin <= 60) return `${asMin}m ${floorSec}s`;
		const asHour = Math.floor(duration.asHours());
		const floorMin = Math.floor(duration.minutes());
		if (asHour <= 24) return `${asHour}h ${floorMin}m ${floorSec}s`;
		const asDay = Math.floor(duration.asDays());
		const floorHour = Math.floor(duration.hours());
		return `${asDay}d ${floorHour}h ${floorMin}m ${floorSec}s`;
	}

	onMount(() => {
		updateClientInfo();
		let timer = setInterval(updateClientInfo, 1000);
		// onDestroy handler
		return () => clearInterval(timer);
	});
</script>

<StickyList>
	<StickySlot styled={false}>
		<StickyHeader title="Info" />
	</StickySlot>
	<div class="descGroup">
		<div class="dataLine headLine">
			<TsIcon type="client" source={{ icon: $client.icon }} {connection} />
			<ClientName client={$client} />
			<span class="countryFlag" title={$client.countryCode}>{countryCodeToEmojis($client.countryCode)}</span>
			<div style="flex: 1;" />
			{#if $client.optionalData !== null}
				<PlatformIcon platform={$client.optionalData.platform} version={$client.optionalData.version} />
			{/if}
		</div>

		<div class="descTable">
			<div>Description:</div>
			<div>{$client.description}</div>
			<div>Online:</div>
			<div>{formatDuration($client.connectionData?.connectedTime)}</div>
			<div>Last active:</div>
			<div>{formatAgo($client.connectionData?.idleTime, true)}</div>
		</div>
		{#if avatarPath}<img class="clientAvatar" src={avatarPath} alt="Client avatar" />{/if}
		<div class="serverGroups">
			<div>Server Groups:</div>
			<div class="serverGroupList">
				{#each groups as grp (grp)}
					<label class="checkbox serverGroupContainer" for={'group' + grp.inner.id}>
						<div class="serverGroupCheckbox">
							<input
								type="checkbox"
								class="checkbox-switch is-info"
								id={'group' + grp.inner.id}
								on:input={(e) => changeServerGroup(e, grp.inner.id, !grp.isMember)}
								checked={grp.isMember} />
						</div>
						<div class="serverGroupSpacing" />
						<div class="serverGroupIcon">
							<ServerGroupIcon id={grp.inner.id} {connection} />
						</div>
						<div class="serverGroupDescription" title={'Id ' + grp.inner.id}>{grp.inner.name}</div>
					</label>
				{/each}
			</div>
		</div>
	</div>
	{#if $developMode || !ownClient}
		<StickySlot>Actions</StickySlot>
		<div class="descGroup">
			<p class="buttons">
				<button class="button is-small is-primary" on:click={onPokeClick}>
					<Icon name="hand-pointing-right" />
					<span>Poke</span>
				</button>
				<button class="button is-small is-warning" on:click={kickFromChannel}>
					<Icon name="shoe-formal" />
					<span>Kick Channel</span>
				</button>
				<button class="button is-small is-danger" on:click={kickFromServer}>
					<Icon name="shoe-formal" />
					<span>Kick Server</span>
				</button>
				<button class="button is-small is-danger">
					<Icon name="cancel" />
					<span>Ban</span>
				</button>
			</p>
			<div class="dataLine">
				<div>Volume:</div>
				<ClientVolume {client} {connection} />
			</div>
			<form on:submit|preventDefault={onPokeSend}>
				<BModal bind:visible={pokeModalVisible}>
					<div slot="header">
						<span>Poke</span>
						<ClientName client={$client} />
					</div>
					<input
						class="input pokeInput"
						type="text"
						bind:this={pokeInput}
						bind:value={pokeMessage} />
					<button type="submit" slot="footer" class="button is-success">Poke</button>
				</BModal>
			</form>
		</div>
	{/if}
	<StickySlot on:click={() => (statsOpen = true)}>
		<button class="button iconButton" on:click|stopPropagation={() => (statsOpen = !statsOpen)}>
			<Icon name="chevron-right{statsOpen ? ' mdi-rotate-90' : ''}" />
		</button>
		<span>Stats</span>
	</StickySlot>
	{#if statsOpen}
		<div class="descGroup">
			<div class="descTable">
				<div>Ping:</div>
				<div>{formatClientPing($client)}</div>
				<div>IP Address:</div>
				<div>{$client.connectionData?.clientAddress ?? ''}</div>
			</div>
		</div>
		<div class="descGroup">
			<BChart bind:this={chart} config={chartConfig} />
		</div>
		<div class="descGroup">
			<div class="statsTable">
				<div />
				<div>Total</div>
				<div>In</div>
				<div>Out</div>

				<div>Packet loss:</div>
				<div>
					{formatPacketLoss($client.connectionData?.serverToClientPacketlossTotal, $client.connectionData?.clientToServerPacketlossTotal)}
				</div>
				<div>{formatPacketLoss($client.connectionData?.serverToClientPacketlossTotal)}</div>
				<div>{formatPacketLoss($client.connectionData?.clientToServerPacketlossTotal)}</div>

				<div>Packets transferred:</div>
				<div />
				<div>
					{formatPacketCount($client.connectionData?.packetsReceivedSpeech, $client.connectionData?.packetsReceivedKeepalive, $client.connectionData?.packetsReceivedControl)}
				</div>
				<div>
					{formatPacketCount($client.connectionData?.packetsSentSpeech, $client.connectionData?.packetsSentKeepalive, $client.connectionData?.packetsSentControl)}
				</div>
			</div>
		</div>
	{/if}
</StickyList>

<style lang="scss">
	.statsTable {
		display: grid;
		grid-template-columns: repeat(4, max-content);
		gap: 0.5em;
	}

	.statsTable > *:nth-child(-n + 4) {
		font-style: italic;
	}

	.countryFlag {
		margin-left: 0.5em;
	}

	.clientAvatar {
		max-width: 100%;
	}

	.serverGroups {
		margin-top: 1.5em;
	}

	.serverGroupList {
		display: table;
		border-spacing: 0 0.1em;
		margin-top: 0.5em;
	}

	.serverGroupContainer {
		display: table-row;
		height: 1.9em;
	}

	.serverGroupContainer > * {
		display: table-cell;
		padding: 0.2em;
		background-color: mix($background, $text, 95%);
		vertical-align: middle;
	}

	.serverGroupCheckbox {
		margin-right: 0.1em;
		border-radius: 0.2em 0 0 0.2em;

		input {
			vertical-align: middle;
		}
	}

	.serverGroupSpacing {
		width: 0.1em;
		background-color: transparent;
		padding: 0;
	}

	.serverGroupDescription {
		padding-right: 1em;
		border-radius: 0 0.2em 0.2em 0;
	}

	.pokeInput {
		width: 100%;
	}
</style>
