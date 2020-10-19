<script lang="typescript">
	import Icon from "../ui/Icon.svelte";

	export let videoSrc: string;
</script>

<div class="chatVideo">
	<video controls>
		<source src={videoSrc} />
		<track kind="captions" />
		Your browser does not support the video tag.
	</video>
	<div class="videoTools">
		<button class="videoButton" title="Sync video playback">
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
</style>
