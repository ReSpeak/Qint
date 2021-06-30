<script lang="ts">
	import moment from "moment";
	import type { Duration } from "moment";

	export let duration: Duration;

	let s = duration.seconds();
	let m = duration.minutes();
	let h = duration.hours();

	$: duration = moment.duration({
		seconds: s,
		minutes: m,
		hours: h,
	});
</script>

<div class="field is-horizontal">
	<div class="field">
		<div class="control"><input class="input" type="number" bind:value={h} min={0} /></div>
		<p class="help">Hours</p>
	</div>
	<div class="field">
		<div class="control">
			<input class="input" type="number" bind:value={m} min={0} max={59} />
		</div>
		<p class="help">Minutes</p>
	</div>
	<div class="field">
		<div class="control">
			<input class="input" type="number" bind:value={s} min={0} max={59} />
		</div>
		<p class="help">Seconds</p>
	</div>
</div>
