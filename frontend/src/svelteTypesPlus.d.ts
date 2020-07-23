import * as svelte_store from "svelte/store";

declare module "svelte/store" {
	function get<T>(store: svelte_store.Readable<T>): T;
}

declare namespace JSX {
	type EventHandler<E = Event, T = HTMLElement> = (event: E & { target: EventTarget & T}) => any;
	interface DOMAttributes<T> {
		ondblclick?: EventHandler<MouseEvent, T>;
	}
}
