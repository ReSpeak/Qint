<script>
	import self from "./connect";
	import Icon from "./ui/Icon.svelte";
	import { SERVER_ICON, CLIENT_ICON } from "./ui/const";

	export let connection;
	let data = new self(connection);

	function connect_submit(e) {
		e.preventDefault();
		data.connect();
	}
</script>

<div class="connect-container">
	<div class="inner-connect-container">
		<div class="connect-blur"></div>
		<form class="connect-form" on:submit="{connect_submit}">
			<div>
				<p class="control has-icons-left">
					<input
						bind:value="{data.username}"
						name="username"
						id="username"
						class="input"
						type="text"
						placeholder="Username"
					/>
					<Icon name="{CLIENT_ICON}" is_left />
				</p>
			</div>
			<div>
				<p class="control has-icons-left">
					<input
						bind:value="{data.address}"
						name="server"
						id="server"
						class="input"
						type="text"
						placeholder="Server"
					/>
					<Icon name="{SERVER_ICON}" is_left />
				</p>
			</div>
			<div>
				<button class="button is-primary" name="connect" type="submit">
					Connect
				</button>
			</div>
		</form>
	</div>
</div>

<style lang="scss">
	.connect-container {
		/* https://www.goodfreephotos.com/other-landscapes/scenic-view-of-the-mountains-and-pond-landscape.jpg.php */
		background: url("/background.jpg") repeat fixed center center / cover;
		position: absolute;
		width: 100%;
		height: 100%;
	}

	.inner-connect-container {
		max-width: 100%;
		width: 40em;

		position: relative;
		top: 10%;
		margin: auto auto;
	}

	.connect-blur::before {
		filter: blur(5px);
		background: url("/background.jpg") repeat fixed center center / cover;
		content: "";
		position: absolute;
		top: 0;
		left: 0;
		width: 100%;
		height: 100%;
	}

	.connect-blur {
		position: absolute;
		top: 0;
		bottom: 0;
		left: 0;
		right: 0;
		background-color: #dffc;
		border-radius: 0.4em;
		box-shadow: 0 0.3em 0.3em #0005;
	}

	.connect-form {
		position: relative;
		background-color: #eefa;
		border-radius: 0.4em;
		box-shadow: 0 0.3em 0.3em #0005;
		padding: 0.5em;
	}

	@media (min-width: 35em) {
		.connect-form {
			padding: 4em 8em 4em 8em;
		}
	}

	.connect-form > div {
		box-sizing: border-box;
		left: 0;
		right: 0;
		margin: 1em;
	}

	.connect-form > div input:not([type="checkbox"]),
	.connect-form > div button {
		box-sizing: border-box;
		width: 100%;
	}
</style>
