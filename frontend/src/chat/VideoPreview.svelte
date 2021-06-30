<script lang="ts">
	import Icon from "../ui/icon/Icon.svelte";
	import { HTML5VideoControl, SyncState, YoutubeVideoControl } from "./videoSync";
	import type { IVideoControl } from "./videoSync";
	import { onDestroy, tick } from "svelte";
	import type { NodeSelection } from "../app";
	import { assert, youtubeUrlRegex } from "../util";
	import type { EmbedTypes } from "./previewAnalyzer";
	import debug from "debug";
	const log = debug("VIDEO");

	export let videoSrc: string;
	export let nodeSel: NodeSelection | undefined;
	export let embed: EmbedTypes | undefined;

	let preview_only = true;
	let html5videoElem: HTMLVideoElement | undefined;
	let youtubeVideoElem: HTMLIFrameElement | undefined;
	let videoControl: IVideoControl | undefined | null;
	let vSync: SyncState | undefined;
	let additionalData: string;

	let detectedType: "youtube" | "media";
	let video_key: string;

	additionalData = "";
	const ytMatch = youtubeUrlRegex.exec(videoSrc);
	if (ytMatch !== null) {
		detectedType = "youtube";
		video_key = ytMatch[5];
		if (ytMatch[6] !== undefined) {
			const params = new Map<string, string>();
			const queryStr = ytMatch[6].startsWith("?") ? ytMatch[6].substring(1) : ytMatch[6];
			const queryParams = queryStr.split(/(&|\?)/g);
			for (const param of queryParams) {
				if (param.includes("=")) {
					const [key, value] = param.split(/=/, 2);
					params.set(key, value);
				}
			}
			const startTime = params.get("t") ?? params.get("start");
			if (startTime !== undefined) {
				if (/^\d+$/.test(startTime)) {
					additionalData += `&start=${startTime}`;
				} else {
					const ts = /^((\d+)h)?((\d+)m)?((\d+)s)?$/.exec(startTime);
					if (ts !== null) {
						const seconds =
							Number(ts[2] ?? 0) * 3600 +
							Number(ts[4] ?? 0) * 60 +
							Number(ts[6] ?? 0);
						additionalData += `&start=${seconds}`;
					}
				}
			}
		}
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
		if (nodeSel === undefined) return;
		if (preview_only) {
			preview_only = false;
			await tick();
		}

		if (vSync === undefined) {
			const _videoControl = getVideoControl();
			if (_videoControl === null) return;
			log("key:%s %o %o", video_key, nodeSel, videoControl);
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

<div class="chatVideoWrap padTop">
	<div class="chatVideo limitChatSize">
		{#if detectedType === "media"}
			<!-- svelte-ignore a11y-media-has-caption -->
			<video bind:this={html5videoElem} controls playsinline allowfullscreen>
				<source src={videoSrc} />
				Your browser does not support the video tag.
			</video>
		{:else if detectedType === "youtube"}
			{#if preview_only}
				<div class="playableOverlay" on:click={() => (preview_only = false)}>
					<img
						class="fixedSize"
						src="https://i.ytimg.com/vi/{video_key}/mqdefault.jpg"
						alt="Click to load video" />
					<div class="playButton">
						<Icon name="play" size="2em" />
					</div>
				</div>
			{:else}
				<iframe
					bind:this={youtubeVideoElem}
					class="fixedSize"
					title="Youtube Video"
					type="text/html"
					src="https://www.youtube.com/embed/{video_key}?enablejsapi=1&rel=0&modestbranding=1&playsinline=1&controls=1&autoplay=1{additionalData}"
					frameborder="0"
					allowfullscreen
					playsinline />
			{/if}
		{/if}
		<div class="videoTools">
			{#if nodeSel !== undefined}
				<button
					class="videoButton"
					class:syncOn={vSync?.enabled}
					title="Sync video playback"
					on:click={toggleVSync}>
					<Icon name="account-multiple" />
				</button>
			{/if}
			<a
				class="videoButton"
				href={videoSrc}
				target="_blank"
				title="Open original link in new tab">
				<Icon name="open-in-new" />
			</a>
		</div>
	</div>
</div>

<style lang="scss">
	// Adjusts the horizontal width of the box (including the command buttons)
	.chatVideoWrap {
		display: flex;
	}

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

	.playableOverlay {
		cursor: pointer;
		display: grid;
		align-items: center;
		justify-content: center;

		> :global(*) {
			grid-area: 1 / 1;
		}

		&:hover .playButton {
			color: red;
		}
	}

	.playButton {
		text-align: center;
		color: black;
		-webkit-text-stroke-width: 3px;
		-webkit-text-stroke-color: white;
		font-size: 5em;

		transition: color 0.2s ease;
	}
</style>
