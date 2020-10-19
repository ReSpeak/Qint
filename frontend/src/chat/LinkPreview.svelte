<script lang="typescript">
	import Loader from "../ui/Loader.svelte";
	import Icon from "../ui/Icon.svelte";
	import VideoPreview from "./VideoPreview.svelte";
	import { analyzeLink } from "./previewAnalyzer";
	import { autoError } from "../util";

	export let link: string;
	export let textContent: string;

	$: analyzeResult = analyzeLink(link);
</script>

<svelte:options immutable={true} />
{#await analyzeResult}
	<Loader text="Loading preview..." />
{:then result}
	{#if result.kind === 'image'}
		<a href={link} target="_blank">
			<img class="limitImg" src={result.imageSrc} alt={textContent} />
		</a>
	{:else if result.kind === 'video'}
		<VideoPreview videoSrc={result.videoSrc} />
	{:else if result.kind === 'site'}
		<a href={link} target="_blank" class="box">
			<div class="media">
				<div class="media-left">
					<figure class="image is-48x48">
						<img use:autoError src={result.imageSrc} alt="Link preview" />
						<!-- <img src={result.imageSrc} alt="Link preview" /> -->
					</figure>
				</div>
				<div class="media-content">
					<p class="title">{result.title}</p>
					<span>{result.description ?? ''}</span>
				</div>
				<div class="media-right">
					<button
						class="button close"
						on:click|stopPropagation|preventDefault={() => {
							result = { kind: undefined };
						}}>
						<Icon name="close" />
					</button>
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
	}

	.close {
		border-radius: 100%;
		width: 2.5em;
		height: 2.5em;
	}
</style>
