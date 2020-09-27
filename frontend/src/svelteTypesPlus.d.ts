import * as svelte_store from "svelte/store";

declare module "svelte/store" {
	function get<T>(store: svelte_store.Readable<T>): T;
}
