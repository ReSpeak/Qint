<script lang="typescript">
	import Icon from "./Icon.svelte";
	import moment from "moment";

	export let platform: string | null;
	export let version: string | null;
	let icon!: string;
	let versionName: string;
	let buildDate: string;
	$: {
		switch (platform) {
			case "Android": icon = "android-debug-bridge"; break;
			case "Windows": icon = "microsoft-windows"; break;
			case "Linux": icon = "linux"; break;
			case "OS X": icon = "apple"; break;
			case "iOS": icon = "apple-ios"; break;
			default: icon = "toaster"; break;
		}
	}
	$: {
		versionName = "";
		buildDate = "";
		if (version !== null) {
			const match = /([^\s]+) \[Build: (\d+)\]/.exec(version);
			if (match !== null) {
				versionName = match[1];
				const num = Number(match[2]);
				if (!Number.isNaN(num)) {
					buildDate = moment.unix(num).toISOString();
				}
			}
		}
	}
</script>

<span title={buildDate}>{versionName}</span>
<Icon name={icon} title={platform ?? ''} />
