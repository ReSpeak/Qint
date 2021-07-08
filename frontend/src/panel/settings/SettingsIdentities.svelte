<script lang="ts">
	import { base64Encode, clickToSelectAll } from "../../util";
	import { backend } from "../../backend/backend";
	import TabSlot from "../../ui/container/TabSlot.svelte";
	import KeyValue from "../../ui/util/KeyValue.svelte";
	import Icon from "../../ui/icon/Icon.svelte";
	import EmojiString from "../../ui/specialized/EmojiString.svelte";
	import FileIO from "../../ui/util/FileIO.svelte";
	import { loadIdentities as liArr } from "./identity";
	import type { ApiIdentity } from "./identity";

	let fileIo: FileIO;
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

	async function clickNewIdentity() {
		try {
			const req = await backend.fetch("/ident/new", {
				method: "POST",
			});
			const newIdentity = await req.json();
			await loadIdentities();

			const newIndex = identities.findIndex((ident) => ident.id === newIdentity.id);
			selectIndex(newIndex);
		} catch (ex) {
			// TODO: change to debug and show on ui
			console.log("Failed to upload: ", ex);
		}
	}

	async function clickImportIdentity(files: CustomEvent<FileList>) {
		const content = await files.detail[0].text();
		await importIdentityFromString(content);
	}

	async function importIdentityFromString(data: string) {
		try {
			// import is special...
			const req = await backend.fetch("/ident/im" + "port", {
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

	async function deleteIdentity() {
		if (editIdentity === undefined) return;
		try {
			const req = await backend.fetch(`/ident/${editIdentity.id}`, {
				method: "DELETE",
			});
			await req.text();
			await loadIdentities();
			editIdentity = undefined;
		} catch (ex) {
			// TODO: change to debug and show on ui
			console.log("Failed to update: ", ex);
		}
	}
</script>

<!-- svelte-ignore a11y-missing-attribute -->
<TabSlot title="Identities">
	<div class="layout">
		<div class="identList panel is-primary">
			<p class="panel-heading">Your Identities</p>

			<a class="panel-block is-active" on:click={() => clickNewIdentity()}>
				<Icon name="plus" />
				New
			</a>

			<a class="panel-block is-active" on:click={() => fileIo.askUpload()}>
				<Icon name="file-upload-outline" />
				Import
			</a>

			<div class="panel-block" style="padding: 0" />

			<div class="identItems">
				{#each identities as identity, index}
					<a
						style="color:{identity.color};"
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
				<KeyValue label="Name" labelStyle="is-normal">
					<div class="is-horizontal field">
						<input type="text" bind:value={editIdentity.name} class="input" />
					</div>
				</KeyValue>

				<KeyValue label="Uid" labelStyle="is-normal">
					<div class="field has-addons">
						<p class="control has-icons-right" style="flex: 1;">
							<span class="input" use:clickToSelectAll>
								{base64Encode(editIdentity.uid)}
							</span>
							<Icon name="lock-outline" isRight />
						</p>
					</div>
				</KeyValue>

				<KeyValue label="Uid (Emoji)" labelStyle="is-normal">
					<div class="field has-addons">
						<p class="control has-icons-right" style="flex: 1;">
							<span class="input" use:clickToSelectAll>
								<EmojiString data={editIdentity.uid} />
							</span>
							<Icon name="lock-outline" isRight />
						</p>
					</div>
				</KeyValue>

				<KeyValue label="Security Level" labelStyle="is-normal">
					<div class="field has-addons">
						<p class="control has-icons-right" style="flex: 1;">
							<span class="input" use:clickToSelectAll>
								{editIdentity.level}
							</span>
							<Icon name="lock-outline" isRight />
						</p>
					</div>
				</KeyValue>

				<!-- <button title="Import a identity" on:click={() => dummyUploader.click()} class="button">
				<Icon name="file-import-outline" />
				<span>Import</span>
				TODO add dropdown with
				Import AS:
				- Teampseak file
				- Any string
				/button> -->

				<KeyValue label="">
					<p class="buttons is-right">
						<button
							type="button"
							class="button is-danger"
							on:click={() => deleteIdentity()}>
							<Icon name="delete" />
							<span>Delete</span>
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
				</KeyValue>
			{/if}
		</form>
	</div>

	<FileIO bind:this={fileIo} on:uploadRequest={clickImportIdentity} />
</TabSlot>

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
