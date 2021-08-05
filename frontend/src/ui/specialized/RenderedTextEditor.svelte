<script lang="ts">
	import RenderedText from "./RenderedText.svelte";
	import ChatInput from "../specialized/ChatInput.svelte";
	import Icon from "../icon/Icon.svelte";
	import { debounced } from "../../util";
	import { backend } from "../../backend/backend";
	import type { IMarkdownTransform } from "../../backend/backend";
	import { onMount } from "svelte";
	import type { Connection } from "../../connection";

	const enum View {
		Edit,
		Both,
		Rendered,
	}

	export let connection: Connection;
	export let raw: string;

	let view: View = View.Edit;
	let rendered: string = "";
	let mdRenderSocket: IMarkdownTransform | undefined;
	$: renderRequest(raw);

	const renderRequest = debounced(
		(text: string) => {
			try {
				mdRenderSocket?.write(text).then((r) => (rendered = r));
			} catch (e) {
				console.error("Failed to render text", e);
			}
		},
		100,
		{
			resetOnCall: false,
		}
	);

	onMount(() => {
		mdRenderSocket = backend.get_markdown_transformer();
		return () => {
			mdRenderSocket?.close();
			mdRenderSocket = undefined;
		};
	});
</script>

<div class="field has-addons">
	<p class="control">
		<button class="button" on:click={() => (view = View.Edit)}>
			<Icon name="pencil" title="Source" />
		</button>
	</p>
	<p class="control">
		<button class="button" on:click={() => (view = View.Both)}>
			<Icon name="flip-horizontal" title="Split view" />
		</button>
	</p>
	<p class="control">
		<button class="button" on:click={() => (view = View.Rendered)}>
			<Icon name="eye" title="Preview" />
		</button>
	</p>
</div>

<div class="editbox">
	{#if view === View.Edit || view === View.Both}
		<ChatInput enterToSubmit={false} bind:value={raw} />
	{/if}
	{#if view === View.Rendered || view === View.Both}
		<div class="renderSide">
			<RenderedText {connection} text={rendered} />
		</div>
	{/if}
</div>

<style lang="scss">
	.editbox {
		display: flex;

		> :global(*) {
			flex-grow: 1;
			flex-basis: 0;
		}
	}

	.renderSide {
		overflow: hidden;
		padding: 0.5em;
	}
</style>
