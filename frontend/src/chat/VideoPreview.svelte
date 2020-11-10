<script lang="typescript">
	import Icon from "../ui/Icon.svelte";
	import type { Connection } from "../connection";
	import { HTML5VideoControl, SyncState } from "./videoSync";
	import type { IVideoControl, VSyncEvent } from "./videoSync";
	import { onDestroy, onMount } from "svelte";
	import type { IMsgPluginCommandPart } from "../book_events";
	import type { NodeSelection } from "../app";

	export let videoSrc: string;
	export let nodeSel: NodeSelection;

	let html5videoElem: HTMLVideoElement | undefined;
	type CacheType = { videoControl: IVideoControl; video_key: string };
	let cache: CacheType | undefined | null;
	let vSync: SyncState | undefined;

	function getVideoControl(): CacheType | null {
		if (cache !== undefined) return cache;
		if (html5videoElem) {
			cache = {
				videoControl: new HTML5VideoControl(html5videoElem),
				video_key: videoSrc,
			};
		} else {
			cache = null;
		}
		return cache;
	}

	function toggleVSync() {
		if (vSync === undefined) {
			let videoData = getVideoControl();
			if (videoData === null) return;
			console.log(nodeSel, videoData.video_key, videoData.videoControl);
			vSync = new SyncState(nodeSel, videoData.video_key, videoData.videoControl);
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
	<video bind:this={html5videoElem} controls>
		<source src={videoSrc} />
		<track kind="captions" />
		Your browser does not support the video tag.
	</video>
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
</style>
