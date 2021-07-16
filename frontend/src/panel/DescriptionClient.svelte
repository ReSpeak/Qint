<script lang="ts">
	import ImageModal from "../chat/ImageModal.svelte";
	import { get } from "svelte/store";
	import { Connection } from "../connection";
	import type { ChangePromise } from "../connection";
	import type { ServerGroupId } from "../ts";
	import moment from "moment";
	import type { Duration, Moment } from "moment";
	import Icon from "../ui/icon/Icon.svelte";
	import PlatformIcon from "../ui/icon/PlatformIcon.svelte";
	import ServerGroupIcon from "../ui/icon/ServerGroupIcon.svelte";
	import TsIcon from "../ui/icon/TsIcon.svelte";
	import ClientName from "../ui/name/ClientName.svelte";
	import StickyList from "../ui/container/StickyList.svelte";
	import StickySlot from "../ui/container/StickySlot.svelte";
	import StickyHeader from "./StickyHeader.svelte";
	import ClientVolume from "../ui/specialized/ClientVolume.svelte";
	import { getClientAvatarPath } from "../ui/icon/tsIcons";
	import { Reason } from "../book_events";
	import { onMount } from "svelte";
	import {
		formatDuration,
		formatSi,
		hexEncode,
		iconPathToId,
		LONG_DATETIME,
		NARROW_NO_BREAK_SPACE,
		on,
	} from "../util";
	import { Client, ServerGroup } from "../book";
	import Modal from "../ui/container/Modal.svelte";
	import { tick } from "svelte";
	import Chart from "../ui/html/Chart.svelte";
	import ChangeResult from "../ui/specialized/ChangeResult.svelte";
	import EmojiString from "../ui/specialized/EmojiString.svelte";
	import ImageFileBrowser from "./ImageFileBrowser.svelte";
	import { app } from "../app";
	import type { ChartConfiguration } from "chart.js";
	import "chartjs-adapter-moment";

	export let connection: Connection;
	export let client: Client;

	const CHART_ENTRY_COUNT = 61;

	let statsOpen = false;
	let pokeModalVisible = false;
	let pokeInput: HTMLElement | undefined;
	let pokeMessage: string = "";
	const developMode = app.transientSettings.ui._developMode;
	let chart: Chart | null = null;
	let editing = false;
	let editIcon = false;
	let dummyUploader: HTMLInputElement;
	let timer: number | undefined;
	let showBigAvatar = false;

	const serverGroups = connection.book.serverGroups;
	let avatarPath: string | undefined;
	$: getClientAvatarPath(connection, $client).then((path) => (avatarPath = path));
	$: ownClient = client.id === connection.book.ownClientId;
	$: {
		if ($client.optionalData == null) getOptionalData();
		if ($client.connectionData == null) getConnectionData();
	}
	$: on(client, onClientChanged());

	$: on(statsOpen, updateTimer());

	// THIS IS NOT A FULL CLIENT OBJECT
	type EditProps = {
		description: string;
	};
	// Can only be changed for own client or overwritten locally for other clients
	type SpecialEditProps = {
		name: string;
		phoneticName: string;
		isChannelCommander: boolean;
	};
	let [clientEdit, clientSpecialEdit] = createPropsCopy();
	let changeRequest: ChangePromise | undefined;
	let iconSelection: string | undefined = undefined;

	type TimeDataPoint = { x: Moment; y: number | undefined };
	const chartConfig: ChartConfiguration<"line", TimeDataPoint[]> = {
		type: "line",
		data: {
			datasets: [
				{
					label: "Ping",
					yAxisID: "ping",
					data: [],
					backgroundColor: "#87A23600",
					borderColor: "#87A236FF",
					pointRadius: 1,
				},
				{
					label: "Packet loss to Server",
					yAxisID: "packetloss",
					data: [],
					backgroundColor: "#9F354800",
					borderColor: "#9F3548FF",
					pointRadius: 1,
				},
				{
					label: "Packet loss from Server",
					yAxisID: "packetloss",
					data: [],
					backgroundColor: "#512C7300",
					borderColor: "#512C73FF",
					pointRadius: 1,
				},
			],
		},
		options: {
			animations: {
				numbers: {
					properties: ["x", "borderWidth", "radius", "tension"],
					type: "number",
				},
			},
			scales: {
				x: {
					type: "time",
					ticks: {
						callback: function (value) {
							let seconds = moment().diff(moment(value, "X"), "seconds");
							if (seconds < 1) seconds = 0;
							return `${seconds}${NARROW_NO_BREAK_SPACE}s`;
						},
					},
					time: {
						unit: "second",
						displayFormats: {
							second: "X",
						},
						stepSize: 5,
					},
				},
				ping: {
					axis: "y",
					beginAtZero: true,
					suggestedMax: 100,
					ticks: {
						maxTicksLimit: 5,
						callback: function (value) {
							return `${value}${NARROW_NO_BREAK_SPACE}ms`;
						},
					},
				},
				packetloss: {
					axis: "y",
					beginAtZero: true,
					suggestedMax: 5,
					ticks: {
						maxTicksLimit: 5,
						callback: function (value) {
							return `${Number(value)}${NARROW_NO_BREAK_SPACE}%`;
						},
					},
				},
			},
		},
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

	function createDataPoint(value: number | undefined): TimeDataPoint {
		return {
			x: moment(),
			y: value,
		};
	}

	function packetLossToPercent(loss01: number | null | undefined): number | undefined {
		if (!loss01) return undefined;
		return loss01 * 100;
	}

	function addChartValue(data: TimeDataPoint[], newValue: number | undefined) {
		data.push(createDataPoint(newValue));

		while (data.length > CHART_ENTRY_COUNT) {
			data.shift();
		}
	}

	function chartRefresh() {
		addChartValue(
			chartConfig.data.datasets[0].data,
			client.connectionData?.ping?.asMilliseconds()
		);
		addChartValue(
			chartConfig.data.datasets[1].data,
			packetLossToPercent(client.connectionData?.clientToServerPacketlossTotal)
		);
		addChartValue(
			chartConfig.data.datasets[2].data,
			packetLossToPercent(client.connectionData?.serverToClientPacketlossTotal)
		);
		chart?.updateChart();
	}

	function onClientChanged() {
		editing = false;
		chartConfig.data.datasets.forEach((dataset) => {
			dataset.data = [];
			for (let i = CHART_ENTRY_COUNT; i > 0; i--) {
				const entry = {
					x: moment().subtract(i, "second"),
					y: undefined,
				};
				dataset.data.push(entry);
			}
		});
		chart?.updateChart();
	}

	async function changeServerGroup(e: Event, group: ServerGroupId, isMember: boolean) {
		if (e.target instanceof HTMLInputElement) e.target.disabled = true;

		if (isMember) {
			// TODO Handle result
			await connection.sendChange({
				ClientAddServerGroup: {
					id: client.id,
					serverGroup: group,
				},
			});
		} else {
			// TODO Handle result
			await connection.sendChange({
				ClientRemoveServerGroup: {
					id: client.id,
					serverGroup: group,
				},
			});
		}
	}

	async function kickFromChannel() {
		// TODO Handle result
		await connection.sendChange({
			ClientKick: {
				id: client.id,
				reason: Reason.KickChannel,
			},
		});
	}

	async function kickFromServer() {
		// TODO Handle result
		await connection.sendChange({
			ClientKick: {
				id: client.id,
				reason: Reason.KickServer,
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
		if (
			client.connectionData === null ||
			client.connectionData.ping === null ||
			client.connectionData.pingDeviation === null
		)
			return "";

		return `(${Math.round(client.connectionData.ping.asMilliseconds() * 10) / 10} ± ${
			Math.round(client.connectionData.pingDeviation.asMilliseconds() * 10) / 10
		})${NARROW_NO_BREAK_SPACE}ms`;
	}

	function formatPacketLoss(...losses: (number | null | undefined)[]) {
		const filteredLosses = losses.flatMap((a) => (a !== null && a !== undefined ? [a] : []));
		if (filteredLosses.length === 0) {
			return "unknown";
		} else {
			const reduced = packetLossToPercent(filteredLosses.reduce((a, b) => a + b)) ?? 0;
			return `${reduced.toFixed(1)}${NARROW_NO_BREAK_SPACE}%`;
		}
	}

	function formatPacketCount(...packetCounts: (string | null | undefined)[]) {
		return formatSi(
			packetCounts.map((x) => (x ? parseInt(x) : 0)).reduce((a, b) => a + b),
			1
		);
	}

	function formatAgo(duration: Duration | null | undefined, ago: boolean): string {
		if (!duration) return "";
		if (ago) return moment.duration(-duration.asSeconds(), "seconds").humanize(true);
		else return moment.duration(duration.asSeconds(), "seconds").humanize();
	}

	async function uploadSelectedAvatar() {
		const files = dummyUploader.files;
		if (files && files.length > 0) {
			const file = files[0];
			dummyUploader.value = null!;
			await connection.backend.fetch("/file/0/avatar", {
				method: "PUT",
				body: file,
			});
			let hash = file.size.toString();
			// Use SHA-256 hash and fall back to file size if not available
			if (crypto.subtle) {
				const hashBuffer = await crypto.subtle.digest("SHA-256", await file.arrayBuffer());
				hash = hexEncode(Array.from(new Uint8Array(hashBuffer)));
			}
			changeRequest = connection.sendChange({
				ConnectionClientUpdate: {
					avatarHash: hash,
				},
			});
		}
	}

	function deleteAvatar() {
		changeRequest = connection.sendChange({
			ServerDeleteFile: {
				path: `/avatar_${client.uidStr}`,
			},
		});
	}

	function createPropsCopy(): [EditProps, SpecialEditProps] {
		return [
			{
				description: client.description,
			},
			{
				name: client.name,
				phoneticName: client.phoneticName,
				isChannelCommander: client.isChannelCommander,
			},
		];
	}

	function getPropsDiff() {
		const diff: Record<string, any> = {};
		for (const [key, value] of Object.entries(clientEdit)) {
			if (key.startsWith("_")) continue;
			if ((client as any)[key] !== value) {
				diff[key] = value;
			}
		}
		return diff;
	}

	function getSpecialPropsDiff() {
		const diff: Record<string, any> = {};
		for (const [key, value] of Object.entries(clientSpecialEdit)) {
			if (key.startsWith("_")) continue;
			if ((client as any)[key] !== value) {
				diff[key] = value;
			}
		}
		return diff;
	}

	function clickEditMode() {
		editing = true;
		const [e, specialE] = createPropsCopy();
		clientEdit = e;
		clientSpecialEdit = specialE;
		iconSelection = "icon_" + client.icon;
	}

	function clickSaveChanges() {
		editing = false;

		let diff = getPropsDiff();
		if (Object.keys(diff).length !== 0) {
			changeRequest = connection.sendChange({
				ClientEdit: {
					id: client.id,
					...diff,
				},
			});
		}

		diff = getSpecialPropsDiff();
		if (Object.keys(diff).length !== 0) {
			if (ownClient) {
				changeRequest = connection.sendChange({
					ConnectionClientUpdate: {
						...diff,
					},
				});
			} else {
				// TODO Save custom name and phonetic name for other clients
			}
		}

		const newIcon = iconPathToId(iconSelection);
		if (newIcon !== client.icon) {
			changeRequest = connection.sendChange({
				ClientAddPerm: {
					id: client.id,
					permissionName: "i_icon_id",
					value: parseInt(newIcon) >> 0, // Cast to signed i32, icon ids are u32s but permission values are i32s
					skip: false,
				},
			});
		}
	}

	function updateTimer() {
		if (timer !== undefined) clearInterval(timer);
		// Throttle when stats are not open, we still need to update last active and online time
		timer = window.setInterval(updateClientInfo, statsOpen ? 1000 : 10000);
	}

	onMount(() => {
		updateClientInfo();
		// onDestroy handler
		return () => {
			if (timer !== undefined) clearInterval(timer);
		};
	});
</script>

{#if showBigAvatar && avatarPath}
	<ImageModal src={avatarPath} bind:visible={showBigAvatar} />
{/if}
<StickyList>
	<StickySlot styled={false}>
		<StickyHeader title="Info">
			{#if editing}
				<button
					class="button is-small is-success"
					on:click|stopPropagation={clickSaveChanges}>
					<Icon name="check" />
					<span>Save</span>
				</button>
				<button
					class="button is-small is-danger"
					on:click|stopPropagation={() => (editing = false)}>
					<Icon name="close" />
					<span>Cancel</span>
				</button>
			{:else}
				<button
					class="button is-small outline-button"
					on:click|stopPropagation={clickEditMode}>
					<Icon name="pencil" />
					<span>Edit</span>
				</button>
			{/if}
		</StickyHeader>
	</StickySlot>
	<div class="descGroup" class:editing>
		{#await changeRequest then changeResult}
			{#if changeResult !== undefined}
				<div class="notification is-danger">
					<button
						class="toolbutton is-small"
						style="float: right;"
						on:click={() => (changeRequest = undefined)}>
						<Icon name="close" />
					</button>
					<ChangeResult result={changeResult} />
				</div>
			{/if}
		{/await}

		<div class="dataLine headLine">
			{#if editing}
				<button class="button" on:click={() => (editIcon = !editIcon)}>
					<TsIcon
						type="client"
						source={{ icon: iconPathToId(iconSelection) }}
						{connection} />
				</button>
			{:else}
				<TsIcon type="client" source={{ icon: $client.icon }} {connection} />
			{/if}
			{#if editing}
				{#if !ownClient}
					<Icon
						name="information-outline"
						title="Change is not visible for others"
						style="margin-right: 0.5em;" />
				{/if}
				<input class="input" type="text" bind:value={clientSpecialEdit.name} />
			{:else}
				<ClientName client={$client} />
			{/if}
			<span class="countryFlag" title={$client.countryCode}>
				{countryCodeToEmojis($client.countryCode)}
			</span>
			<div style="flex: 1;" />
			{#if $client.optionalData !== null}
				<PlatformIcon
					platform={$client.optionalData.platform}
					version={$client.optionalData.version} />
			{/if}
		</div>

		{#if editing && editIcon}
			<div class="dataLine">
				<ImageFileBrowser
					{connection}
					path={["0", "icons"]}
					canShowBig={false}
					forSelection={true}
					bind:selection={iconSelection} />
			</div>
		{/if}

		<div class="descTable">
			{#if editing}
				<label for="edit_phoneticName">
					Phonetic name{#if !ownClient}<Icon
							name="information-outline"
							title="Change is not visible for others" />{/if}:
				</label>
				<div>
					<input
						id="edit_phoneticName"
						class="input"
						type="text"
						bind:value={clientSpecialEdit.phoneticName} />
				</div>
			{/if}
			<label for="edit_description">Description:</label>
			<div>
				{#if editing}
					<input
						id="edit_description"
						class="input"
						type="text"
						bind:value={clientEdit.description} />
				{:else}{$client.description}{/if}
			</div>
			<div>Online:</div>
			<div>{formatDuration($client.connectionData?.connectedTime)}</div>
			<div>Last active:</div>
			<div>{formatAgo($client.connectionData?.idleTime, true)}</div>
			{#if $developMode}
				{#if $client.optionalData !== null}
					<div>First connected:</div>
					<div>{$client.optionalData.created.format(LONG_DATETIME)}</div>
					<div>Last connected:</div>
					<div>{$client.optionalData.lastConnected.format(LONG_DATETIME)}</div>
				{/if}

				{#if $client.uid !== null}
					<div>Uid:</div>
					<div>{$client.uidStr}</div>
					<div>Uid (emoji):</div>
					<div>
						<EmojiString data={$client.uid} />
					</div>
				{/if}
				<div>Id:</div>
				<div>{$client.id}</div>
				<div>Database id:</div>
				<div>{$client.databaseId}</div>
			{/if}
			{#if editing}
				<div>
					<label for="client_channel_commander">Channel commander:</label>
				</div>
				<div>
					<input
						id="client_channel_commander"
						type="checkbox"
						class="checkbox-switch is-info"
						bind:checked={clientSpecialEdit.isChannelCommander} />
				</div>
				<div>Avatar:</div>
				<div>
					{#if ownClient}
						<input
							title="Dummy Uploader"
							style="display: none;"
							bind:this={dummyUploader}
							on:change={uploadSelectedAvatar}
							type="file" />
						<button
							class="button is-small is-info"
							on:click={() => dummyUploader.click()}>Upload</button>
					{/if}
					{#if avatarPath}
						<button class="button is-small is-danger" on:click={deleteAvatar}>
							Delete
						</button>
					{/if}
				</div>
			{/if}
		</div>
		{#if avatarPath}
			<img
				class="clientAvatar"
				src={avatarPath}
				alt="Client avatar"
				title="Click to enlarge"
				on:click={() => (showBigAvatar = true)} />
		{/if}
		<div class="serverGroups">
			<div>Server Groups:</div>
			<div class="serverGroupList">
				{#each groups as grp (grp)}
					<label class="checkbox serverGroupContainer" for={"group" + grp.inner.id}>
						<div class="serverGroupCheckbox">
							<input
								type="checkbox"
								class="checkbox-switch is-info"
								id={"group" + grp.inner.id}
								on:input={(e) => changeServerGroup(e, grp.inner.id, !grp.isMember)}
								checked={grp.isMember} />
						</div>
						<div class="serverGroupSpacing" />
						<div class="serverGroupIcon">
							<ServerGroupIcon id={grp.inner.id} {connection} />
						</div>
						<div class="serverGroupDescription" title={"Id " + grp.inner.id}>
							{grp.inner.name}
						</div>
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
				<Modal bind:visible={pokeModalVisible}>
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
				</Modal>
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
				<div>{$client.connectionData?.clientAddress ?? ""}</div>
			</div>
		</div>
		<div class="descGroup">
			<Chart bind:this={chart} config={chartConfig} />
		</div>
		<div class="descGroup">
			<div class="statsTable">
				<div />
				<div>Total</div>
				<div>In</div>
				<div>Out</div>

				<div>Packet loss:</div>
				<div>
					{formatPacketLoss(
						$client.connectionData?.serverToClientPacketlossTotal,
						$client.connectionData?.clientToServerPacketlossTotal
					)}
				</div>
				<div>{formatPacketLoss($client.connectionData?.serverToClientPacketlossTotal)}</div>
				<div>{formatPacketLoss($client.connectionData?.clientToServerPacketlossTotal)}</div>

				<div>Packets transferred:</div>
				<div />
				<div>
					{formatPacketCount(
						$client.connectionData?.packetsReceivedSpeech,
						$client.connectionData?.packetsReceivedKeepalive,
						$client.connectionData?.packetsReceivedControl
					)}
				</div>
				<div>
					{formatPacketCount(
						$client.connectionData?.packetsSentSpeech,
						$client.connectionData?.packetsSentKeepalive,
						$client.connectionData?.packetsSentControl
					)}
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
