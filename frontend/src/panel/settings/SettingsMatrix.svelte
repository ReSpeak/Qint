<script lang="ts">
	import { invoke } from "@tauri-apps/api/core";
	import TabSlot from "../../ui/container/TabSlot.svelte";
	import KeyValue from "../../ui/util/KeyValue.svelte";
	import DropDown from "../../ui/html/DropDown.svelte";
	import { backend } from "../../backend/backend";
	import { matrixStatus, refreshMatrixStatus } from "../../matrix";
	import { IS_TAURI } from "../../util";

	type Room = { id: string; name: string };

	let homeserver = "matrix.org";
	let user = "";
	let password = "";
	let busy = false;
	let errorMsg: string | undefined;
	let ssoUrl: string | undefined;

	$: selectedRoom = $matrixStatus.rooms.find((r) => r.id === $matrixStatus.roomId);

	async function run(fn: () => Promise<void>) {
		if (busy) return;
		busy = true;
		errorMsg = undefined;
		try {
			await fn();
		} catch (e) {
			errorMsg = String(e);
		} finally {
			busy = false;
		}
	}

	const loginPassword = () =>
		run(async () => {
			await backend.matrix_login_password(homeserver, user, password);
			password = "";
		});

	const loginSso = () =>
		run(async () => {
			ssoUrl = await backend.matrix_login_sso(homeserver);
			// Open in the system browser; the login completes via a localhost redirect
			if (IS_TAURI) await invoke("plugin:opener|open_url", { url: ssoUrl });
			else window.open(ssoUrl, "_blank");
		});

	const logout = () =>
		run(async () => {
			ssoUrl = undefined;
			await backend.matrix_logout();
		});

	const setRoom = (ev: CustomEvent<Room>) =>
		run(async () => {
			await backend.matrix_set_room(ev.detail.id);
			await refreshMatrixStatus();
		});
</script>

<TabSlot title="Matrix">
	{#if $matrixStatus.loggedIn}
		<KeyValue label="Account" labelStyle="is-normal">
			<span class="input">{$matrixStatus.userId ?? ""}</span>
		</KeyValue>
		<KeyValue label="Homeserver" labelStyle="is-normal">
			<span class="input">{$matrixStatus.homeserver ?? ""}</span>
		</KeyValue>
		<KeyValue label="Room" labelStyle="is-normal">
			{#if $matrixStatus.rooms.length === 0}
				<span class="input">No joined rooms yet</span>
			{:else}
				<DropDown
					items={$matrixStatus.rooms}
					selected={selectedRoom}
					display={(r) => r.name}
					compare={(a, b) => a.id === b?.id}
					on:change={setRoom}
				/>
			{/if}
		</KeyValue>
		<KeyValue label="">
			<button class="button is-danger" class:is-loading={busy} on:click={logout}>Log out</button>
		</KeyValue>
	{:else}
		<form on:submit|preventDefault={loginPassword}>
			<KeyValue label="Homeserver" labelStyle="is-normal">
				<input type="text" class="input" bind:value={homeserver} required />
			</KeyValue>
			<KeyValue label="User" labelStyle="is-normal">
				<input type="text" class="input" bind:value={user} autocomplete="username" />
			</KeyValue>
			<KeyValue label="Password" labelStyle="is-normal">
				<input
					type="password"
					class="input"
					bind:value={password}
					autocomplete="current-password"
				/>
			</KeyValue>
			<KeyValue label="">
				<p class="buttons">
					<button
						class="button is-success"
						type="submit"
						class:is-loading={busy}
						disabled={!user || !password}>Log in</button
					>
					<button class="button" type="button" class:is-loading={busy} on:click={loginSso}
						>Log in with SSO</button
					>
				</p>
			</KeyValue>
		</form>
		{#if ssoUrl !== undefined}
			<KeyValue label="">
				<p>
					Finish the login in your browser. If it did not open,
					<a href={ssoUrl} target="_blank" rel="noreferrer">open the login page</a>.
				</p>
			</KeyValue>
		{/if}
	{/if}
	{#if errorMsg !== undefined}
		<article class="message is-danger">
			<div class="message-body">{errorMsg}</div>
		</article>
	{/if}
</TabSlot>
