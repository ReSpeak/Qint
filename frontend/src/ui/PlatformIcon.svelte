<script lang="typescript">
	import Icon from "./Icon.svelte";

	export let platform: string;
	let icon!: string;
	$: {
		switch(platform) {
			case "Android": icon = "android-debug-bridge"; break;
			case "Windows": icon = "microsoft-windows"; break;
			case "Linux": icon = "linux"; break;
			case "OS X": icon = "apple"; break;
			case "iOS": icon = "apple-ios"; break;
			default: icon = "toaster"; break;
		}
	}
</script>

<Icon name={icon}/>
