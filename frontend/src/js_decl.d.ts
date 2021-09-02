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

declare module "*.svelte" {
	export { SvelteComponentDev as default } from "svelte/internal";
}

declare const BUILD_ENV: string;
declare const BUILD_DAT: string;
declare const DEBUG_UTIL: boolean;
