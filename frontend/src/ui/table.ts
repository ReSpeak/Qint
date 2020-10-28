export type ColumnKey = string | number;

export type IColumns<T> = IColumn<T>[];

export interface IColumn<T> {
	key: ColumnKey;
	title: string;
	value: (t: T) => any;
	renderValue?: (t: T) => string;
	sortable: boolean;
	headerClass?: string;
	class?: string;
	customRender?: boolean;
	filterMatch?: (t: T, search: string) => boolean;
}

export type IRows<T> = T[];

export const enum SortOrder {
	Desc = -1,
	Asc = 1,
}
