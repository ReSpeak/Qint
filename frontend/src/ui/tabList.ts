import { Readable } from "svelte/store";

export const contextKey: any = {};

export interface TabListContext {
	activeIndex: Readable<number>;
	registerPanel: (title: string) => number;
}

//export interface Svelte
