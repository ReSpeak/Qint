<script lang="ts">
	import { base64Encode, clickToSelectAll } from "../../util";
	import { backend } from "../../backend/backend";
	import BTabSlot from "../../ui/BTabSlot.svelte";
	import BKeyValue from "../../ui/BKeyValue.svelte";
	import Icon from "../../ui/Icon.svelte";
	import UiEmojiString from "../../ui/UiEmojiString.svelte";
	import { loadIdentities as liArr } from "./identity";
	import type { ApiIdentity } from "./identity";

	let dummyUploader: HTMLInputElement;
	let dummyDownloader: HTMLIFrameElement;
	let identities: ApiIdentity[] = [];
	let selectedIndex: number = -1;
	let selectedIdentity: ApiIdentity | undefined;
	let editIdentity: ApiIdentity | undefined;
	$: canSave = selectedIdentity?.name !== editIdentity?.name;

	loadIdentities();

	async function loadIdentities() {
		identities = await liArr();
	}

	function selectIndex(index: number) {
		selectedIndex = index;
		selectedIdentity = identities[selectedIndex];
		editIdentity = selectedIdentity !== undefined ? { ...selectedIdentity } : undefined;
	}

	async function clickImportIdentity() {
		let files = dummyUploader.files;
		if (files && files.length > 0) {
			const content = await files[0].text();
			await importIdentityFromString(content);
			dummyUploader.value = null!;
		}
	}

	async function importIdentityFromString(data: string) {
		try {
			const req = await backend.fetch("/ident/import", {
				method: "POST",
				body: data,
			});
			await req.text();
			await loadIdentities();
		} catch (ex) {
			// TODO: change to debug and show on ui
			console.log("Failed to upload: ", ex);
		}
	}

	async function updateIdentity() {
		console.log("subba");
		if (editIdentity === undefined) return;
		try {
			const req = await backend.fetch(`/ident/${editIdentity.id}?name=${editIdentity.name}`, {
				method: "PUT",
			});
			await req.text();
			await loadIdentities();
		} catch (ex) {
			// TODO: change to debug and show on ui
			console.log("Failed to update: ", ex);
		}
	}
</script>

<!-- svelte-ignore a11y-missing-attribute -->
<BTabSlot title="Identities">
	<div class="layout">
		<div class="identList panel is-primary">
			<p class="panel-heading">Your Identities</p>

			<a class="panel-block is-active">
				<Icon name="plus" />
				New
			</a>

			<a class="panel-block is-active" on:click={() => dummyUploader.click()}>
				<Icon name="file-upload-outline" />
				Import
			</a>

			<div class="panel-block" style="padding: 0" />

			<div class="identItems">
				{#each identities as identity, index}
					<a
						class="panel-block"
						class:is-active={selectedIndex === index}
						on:click={() => {
							selectIndex(index);
						}}>
						<Icon name="account" />
						<span class:isSelected={selectedIndex === index}>{identity.name}</span>
					</a>
				{/each}
			</div>
		</div>

		<form class="identOption" on:submit|preventDefault={updateIdentity}>
			{#if editIdentity !== undefined}
				<BKeyValue label="Name" labelStyle="is-normal">
					<div class="is-horizontal field">
						<input type="text" bind:value={editIdentity.name} class="input" />
					</div>
				</BKeyValue>

				<BKeyValue label="Uid" labelStyle="is-normal">
					<div class="field has-addons">
						<p class="control has-icons-right" style="flex: 1;">
							<span class="input" use:clickToSelectAll>
								{base64Encode(editIdentity.uid)}
							</span>
							<Icon name="lock-outline" isRight />
						</p>
					</div>
				</BKeyValue>

				<BKeyValue label="Uid (Emoji)" labelStyle="is-normal">
					<div class="field has-addons">
						<p class="control has-icons-right" style="flex: 1;">
							<span class="input" use:clickToSelectAll>
								<UiEmojiString data={editIdentity.uid} />
							</span>
							<Icon name="lock-outline" isRight />
						</p>
					</div>
				</BKeyValue>

				<BKeyValue label="Security Level" labelStyle="is-normal">
					<div class="field has-addons">
						<p class="control has-icons-right" style="flex: 1;">
							<span class="input" use:clickToSelectAll>
								{editIdentity.level}
							</span>
							<Icon name="lock-outline" isRight />
						</p>
					</div>
				</BKeyValue>

				<!-- <button title="Import a identity" on:click={() => dummyUploader.click()} class="button">
				<Icon name="file-import-outline" />
				<span>Import</span>
				TODO add dropdown with
				Import AS:
				- Teampseak file
				- Any string
				/button> -->

				<BKeyValue label="">
					<p class="buttons is-right">
						<button class="button is-danger" disabled>
							<Icon name="delete" />
							<span>Delete (Not implemented, do it yourself)</span>
						</button>

						<span style="flex:1;" />

						<button
							class="button is-info"
							title="Export this identity"
							on:click={() => {
								/* TODO */
							}}>
							<Icon name="file-export-outline" />
							<span>Export</span>
							<!-- TODO add dropdown with -->
							<!-- Export AS: -->
							<!-- - Teampseak file -->
							<!-- - Obfuscated string ? -->
						</button>

						<button type="submit" class="button is-success" disabled={!canSave}>
							<Icon name="content-save" />
							<span>Save</span>
						</button>
					</p>
				</BKeyValue>
			{/if}
		</form>
	</div>

	<input
		title="Dummy Uploader"
		style="display: none;"
		bind:this={dummyUploader}
		on:change={clickImportIdentity}
		type="file" />
	<iframe
		title="Dummy Downloader"
		style="display: none;"
		bind:this={dummyDownloader}
		sandbox="allow-downloads" />
</BTabSlot>

<style lang="scss">
	.layout {
		width: 100%;
		height: 100%;
		display: grid;
		grid-template-columns: minmax(max-content, 20em) 1fr;
		grid-template-rows: 1fr;
	}

	.identList {
		overflow-y: hidden;
		display: flex;
		flex-direction: column;
		background-color: $box-background-color;
	}

	.identItems {
		overflow-y: auto;
	}

	.identOption {
		margin-left: 2em;
	}

	.isSelected {
		font-weight: bold;
	}
</style>
