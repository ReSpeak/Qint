import { Readable } from "svelte/store";

export const contextKey: any = {};

export interface TabListContext {
	activeId: Readable<number>;
	registerPanel: (title: string) => number;
	unregisterPanel: (id: number) => void;
}

//export interface Svelte
