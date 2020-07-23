/**
 * Describes whether the list wants elements before or after the given Element
*/
export enum ListFetchDir {
	Before,
	After,
	New,
}

export interface FetchResult<T> {
	items: T[];
	/** true if there are no more elements before the returned items */
	canLoadBeforeStart: boolean;
	/** true if there are no more elements after the returned items */
	canLoadAfterEnd: boolean;
}
