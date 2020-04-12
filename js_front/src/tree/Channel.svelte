<script>
	import { afterUpdate } from "svelte";
	import Icon from "../ui/Icon.svelte";
	import { flash } from "../util";

	export let channel;
	let children = channel.children;

	let collapsed = false;
	// TODO dummy
	let ownClient = false;
	let selectedChannel = false;
	let alignCenter = false;
	let alignRight = false;
	let icon = ""; // TODO

	let div;
	afterUpdate(() => {
		flash(div);
	});
</script>

<li>
	<div
		bind:this={div}
		class="flex-line"
		class:own-client="{ownClient}"
		class:selected-channel="{selectedChannel}"
	>
		<span class="collapse-button" onclick="{(collapsed = !collapsed)}">
			<Icon name="chevron-right {collapsed ? 'mdi-rotate-90' : ''}" />
			{icon}
			<a
				class="expand"
				class:text-align-center="{alignCenter}"
				class:text-align-right="{alignRight}"
			>
				<span class="expand">{channel.name}</span>
			</a>
		</span>
	</div>
	<ul class="menu-list" class:collapsed>
		<!-- Clients -->
		<!-- Channel -->
		{#each $children as channel}
			<svelte:self {channel} />
		{/each}
	</ul>
</li>
