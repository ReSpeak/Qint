export type ColumnKey = string | number;

export type IColumns<T> = IColumn<T>[];

export interface IColumn<T> {
	key: ColumnKey;
	title: string;
	value: (t: T) => any;
	renderValue?: (t: T) => string;
	sort?: (a: T, b: T) => number;
	headerClass?: string;
	class?: string;
	customRender?: boolean;
	filterMatch?: (t: T, search: string) => boolean;
}

export type IRows<T> = T[];

export interface IRowOptions<T> {
	dataType?: (t: T) => string | null;
	dataValue?: (t: T) => string | null;
}

export const enum SortOrder {
	Desc = -1,
	Asc = 1,
}

export type ClickRowEvent<T> = CustomEvent<ClickRowData<T>>;
export type ClickRowData<T> = { event: MouseEvent; row: T; dblclick: boolean };

export interface IDragOptions {
	dropFilter?: () => HTMLElement[];
}
