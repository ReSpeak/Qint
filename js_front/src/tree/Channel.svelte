<script>
	import Channel from "./Channel.svelte";
	import Icon from "../ui/Icon.svelte";

	export let channel;
	let children = channel.children;

	let collapsed = false;
	// TODO dummy
	let ownClient = false;
	let selectedChannel = false;
	let alignCenter = false;
	let alignRight = false;
	let icon = ""; // TODO
</script>

<li>
	<div
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
			<Channel {channel} />
		{/each}
	</ul>
</li>
