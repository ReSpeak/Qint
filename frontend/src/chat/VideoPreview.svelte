<script lang="typescript">
	import Icon from "../ui/Icon.svelte";
	import { HTML5VideoControl, SyncState, YoutubeVideoControl } from "./videoSync";
	import type { IVideoControl } from "./videoSync";
	import { onDestroy, onMount, tick } from "svelte";
	import type { NodeSelection } from "../app";
	import { assert, youtubeUrlRegex } from "../util";
	import type { EmbedTypes } from "./previewAnalyzer";

	export let videoSrc: string;
	export let nodeSel: NodeSelection;
	export let embed: EmbedTypes | undefined;

	let preview_only = true;
	let html5videoElem: HTMLVideoElement | undefined;
	let youtubeVideoElem: HTMLIFrameElement | undefined;
	let videoControl: IVideoControl | undefined | null;
	let vSync: SyncState | undefined;

	let detectedType: "youtube" | "media";
	let video_key: string;

	let ytMatch = youtubeUrlRegex.exec(videoSrc);
	if (ytMatch !== null) {
		detectedType = "youtube";
		video_key = ytMatch[5]; // ?
	} else {
		detectedType = "media";
		video_key = videoSrc;
	}

	function getVideoControl(): IVideoControl | null {
		if (videoControl !== undefined) return videoControl;
		if (detectedType === "media") {
			assert(html5videoElem, "No html5videoElem");
			videoControl = new HTML5VideoControl(html5videoElem);
		} else if (detectedType === "youtube") {
			assert(youtubeVideoElem, "No youtubeVideoElem");
			videoControl = new YoutubeVideoControl(youtubeVideoElem);
		} else {
			videoControl = null;
		}
		return videoControl;
	}

	async function toggleVSync() {
		if (preview_only) {
			preview_only = false;
			await tick();
		}

		if (vSync === undefined) {
			const _videoControl = getVideoControl();
			if (_videoControl === null) return;
			//console.log(nodeSel, video_key, videoControl);
			vSync = new SyncState(nodeSel, video_key, _videoControl);
		}
		if (vSync.enabled) {
			vSync.unsubscribe();
		} else {
			vSync.subscribe();
			vSync.sendJoinOrHost();
		}
		vSync = vSync;
	}
	onDestroy(() => {
		vSync?.unsubscribe();
	});
</script>

<div class="chatVideo">
	{#if detectedType === 'media'}
		<video bind:this={html5videoElem} controls playsinline allowfullscreen>
			<source src={videoSrc} />
			<track kind="captions" />
			Your browser does not support the video tag.
		</video>
	{:else if detectedType === 'youtube'}
		{#if preview_only}
			<img
				class="fixedSize"
				width="640"
				height="390"
				src="https://i.ytimg.com/vi/{video_key}/mqdefault.jpg"
				alt="Click to load video"
				on:click={() => (preview_only = false)} />
		{:else}
			<iframe
				bind:this={youtubeVideoElem}
				class="fixedSize"
				title="Youtube Video"
				type="text/html"
				width="640"
				height="390"
				src="https://www.youtube.com/embed/{video_key}?enablejsapi=1&rel=0&modestbranding=1&playsinline=1&controls=1&autoplay=1"
				frameborder="0"
				allowfullscreen
				playsinline />
		{/if}
	{/if}
	<div class="videoTools">
		<button
			class="videoButton"
			class:syncOn={vSync?.enabled}
			title="Sync video playback"
			on:click={toggleVSync}>
			<Icon name="account-multiple" />
		</button>
		<a class="videoButton" href={videoSrc} target="_blank">
			<Icon name="open-in-new" />
		</a>
	</div>
</div>

<style lang="scss">
	.chatVideo {
		display: flex;
		flex-direction: column;
	}

	video {
		overflow: hidden;
	}

	.videoTools {
		display: flex;
		//border-color: $scheme-main;
		background-color: $grey-accent;
		//border-style: solid solid none solid;
		border-radius: 0 0 1em 1em;
		padding: 0.2em 1em;

		> *:not(:last-child) {
			margin-right: 0.5em;
		}
	}

	.videoButton {
		appearance: none;
		-moz-appearance: none;
		-webkit-appearance: none;

		border: none;
		padding: 0;
		width: 2rem;
		height: 2rem;
		border-radius: 100%;
		background-color: #444444;
		text-align: center;
		cursor: pointer;
		line-height: 1;
		font-size: 1em;

		display: flex;
		justify-content: center;
		align-items: center;
	}

	.syncOn {
		background-color: $info;
	}

	.fixedSize {
		width: 640px;
		height: 390px;
	}
</style>
