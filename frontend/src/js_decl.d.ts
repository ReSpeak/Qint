declare module "@uupaa/dynamic-import-polyfill" {
	export function importModule(url: string): Promise<any>;
}

declare module "chartjs-adapter-moment";

declare namespace svelte.JSX {
	interface HTMLAttributes<T> {
		// Custom drag and drop handler
		onsvddrag?: (e: CustomEvent<DragData>) => void;
		onsvddrop?: (e: CustomEvent<DragData>) => void;
	}
}
