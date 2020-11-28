<script lang="typescript">
	//import Loader from "../ui/Loader.svelte";
	import VideoPreview from "./VideoPreview.svelte";
	import ImageModal from "./ImageModal.svelte";
	import { analyzeLink } from "./previewAnalyzer";
	import { autoError } from "../util";
	import type { NodeSelection } from "../app";

	export let link: string;
	export let textContent: string;
	export let nodeSel: NodeSelection;

	let showBig = false;

	$: analyzeResult = analyzeLink(link);
</script>

<svelte:options immutable={true} />
{#await analyzeResult}
	<!-- <Loader text="Loading preview..." /> -->
{:then result}
	{#if result.kind === 'image'}
		<!-- TODO add 'open original' button -->
		<img
			class="limitChatSize previewImg padTop"
			src={result.imageSrc}
			alt={textContent}
			title="Click to enlage"
			on:click={() => (showBig = true)} />
		{#if showBig}
			<ImageModal src={result.imageSrc} bind:visible={showBig} />
		{/if}
	{:else if result.kind === 'video'}
		<VideoPreview videoSrc={result.videoSrc} embed={result.embed} {nodeSel} />
	{:else if result.kind === 'site'}
		<a href={link} target="_blank" class="box padTop">
			<div>
				<div class="media-left">
					<figure class="image is-48x48">
						<img use:autoError src={result.imageSrc} alt="Link preview" />
					</figure>
				</div>
				<div class="media-content">
					<p class="title">{result.title}</p>
					<span>{result.description ?? ''}</span>
				</div>
			</div>
		</a>
	{/if}
{/await}

<style lang="scss">
	.title {
		font-weight: bold;
		font-size: large;
		margin-bottom: 0.5em;

		text-overflow: ellipsis;
		overflow: hidden;
	}

	.box {
		padding: 1em;
		margin-right: 1em;
	}

	.media-left {
		float: left;
	}
</style>
