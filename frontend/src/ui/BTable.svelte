<!--
Src taken and modified from: https://github.com/dasDaniel/svelte-table

The MIT License (MIT)

Copyright (c) 2019 Daniel Poda

Permission is hereby granted, free of charge, to any person obtaining a copy of
this software and associated documentation files (the "Software"), to deal in
the Software without restriction, including without limitation the rights to
use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of
the Software, and to permit persons to whom the Software is furnished to do so,
subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS
FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR
COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER
IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN
CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.
-->
<script lang="typescript">
	import { createEventDispatcher } from "svelte";
	import { SortOrder } from "./table";
	import type { ColumnKey, IColumn, IColumns, IRows } from "./table";

	const dispatch = createEventDispatcher<{
		clickCol: { event: MouseEvent; col: TCol; key: ColumnKey };
		clickRow: { event: MouseEvent; row: TRow; dblclick: boolean };
		clickCell: { event: MouseEvent; row: TRow; key: ColumnKey };
	}>();

	type TRow = any;
	type TCol = IColumn<TRow>;

	export let columns: IColumns<TRow>;
	export let rows: IRows<TRow>;
	export let sortBy: ColumnKey = "";
	export let sortOrder: SortOrder = SortOrder.Asc;
	export let classNameThead = "";
	export let classNameTbody = "";
	export let classNameRow = "";
	export let classNameCell = "";

	type SortFun = (t: InternalRow) => string | number;
	const defaultSort: SortFun = (t) => t.id;
	let sortFunction: SortFun = defaultSort;
	let columnByKey: Record<ColumnKey, TCol> = {};

	columns.forEach((col) => {
		columnByKey[col.key] = col;
	});

	type InternalRow = {
		t: TRow;
		id: number;
		selected: boolean;
		sortVal?: any;
	};

	let c_rows: InternalRow[];

	$: remap(rows);

	function remap(_rows: IRows<TRow>) {
		selected.clear();
		c_rows = _rows.map((r, id) => {
			return { t: r, selected: false, id };
		});
		// c_rows = c_rows.filter((r) => {
		// })
	}

	$: reSort(sortBy, sortOrder);

	function reSort(_sortBy: ColumnKey, _sortOrder: SortOrder) {
		clearSelection();
		if (_sortBy === "") {
			sortFunction = defaultSort;
		} else {
			c_rows.forEach((r) => {
				r.sortVal = sortFunction(r);
			});
		}
		c_rows = c_rows.sort((a, b) => {
			if (a.sortVal > b.sortVal) return _sortOrder;
			else if (a.sortVal < b.sortVal) return -_sortOrder;
			return 0;
		});
		c_rows.forEach((r, i) => {
			r.id = i;
		});
	}

	$: {
		let col = columnByKey[sortBy];
		if (col !== undefined && col.sortable === true && typeof col.value === "function") {
			sortFunction = (r) => col.value(r.t);
		}
	}

	function updateSortOrder(colKey: ColumnKey) {
		if (colKey === sortBy) {
			sortOrder = sortOrder === 1 ? -1 : 1;
		} else {
			sortOrder = 1;
		}
	}

	let selected = new Set<number>();
	let lastSelected: number = 0;

	function handleClickCol(event: MouseEvent, col: TCol) {
		if (!col.sortable) return;
		updateSortOrder(col.key);
		sortBy = col.key;
		dispatch("clickCol", { event, col, key: col.key });
	}

	function clearSelection() {
		for (const oldSel of selected.values()) {
			c_rows[oldSel].selected = false;
		}
		selected.clear();
		c_rows = c_rows; // refresh list
	}

	function unselectElem(row: InternalRow) {
		if (selected.has(row.id)) {
			selected.delete(row.id);
			row.selected = false;
			c_rows[row.id] = row; // refresh list
		}
	}

	function selectElem(...rows: InternalRow[]) {
		for (const row of rows) {
			if (!selected.has(row.id)) {
				selected.add(row.id);
				row.selected = true;
			}
		}
		c_rows = c_rows; // refresh list
	}

	function toggleElem(row: InternalRow) {
		if (selected.has(row.id)) {
			unselectElem(row);
		} else {
			selectElem(row);
		}
	}

	function handleClickRow(event: MouseEvent, row: InternalRow, dblclick: boolean) {
		let isRealDblClick = dblclick && !event.ctrlKey && !event.shiftKey;
		if (!dblclick) {
			if (event.ctrlKey) {
				lastSelected = row.id;
				toggleElem(row);
				event.preventDefault();
			} else if (event.shiftKey) {
				clearSelection();
				let [start, end] =
					row.id > lastSelected ? [lastSelected, row.id] : [row.id, lastSelected];
				selectElem(...c_rows.slice(start, end + 1));
			} else {
				clearSelection();
				lastSelected = row.id;
				selectElem(row);
			}
		}

		dispatch("clickRow", { event, row: row.t, dblclick: isRealDblClick });
	}

	function handleClickCell(event: MouseEvent, row: InternalRow, key: ColumnKey) {
		dispatch("clickCell", { event, row, key });
	}
</script>

<svelte:window on:click={clearSelection} />
<table on:click|stopPropagation class="table">
	<thead class={classNameThead}>
		<tr>
			{#each columns as col}
				<th
					on:click={(e) => handleClickCol(e, col)}
					class:isSortable={col.sortable}
					class={col.headerClass}>
					{#if col.customRender === true}
						<slot name="headerCell" {col} />
					{:else}{col.title}{/if}
					{#if sortBy === col.key}
						<slot name="orderIcon" {sortOrder}>{sortOrder === 1 ? '▲' : '▼'}</slot>
					{/if}
				</th>
			{/each}
		</tr>
	</thead>
	<tbody class={classNameTbody}>
		{#each c_rows as row}
			<tr
				on:click={(e) => handleClickRow(e, row, false)}
				on:dblclick={(e) => handleClickRow(e, row, true)}
				class:selected={row.selected}
				class={classNameRow}>
				{#each columns as col}
					<td
						on:click={(e) => {
							handleClickCell(e, row, col.key);
						}}
						class={[col.class, classNameCell].join(' ')}>
						{#if col.customRender === true}
							<slot name="colCell" {col} row={row.t} />
						{:else}{col.renderValue ? col.renderValue(row.t) : col.value(row.t)}{/if}
					</td>
				{/each}
			</tr>
		{:else}
			<slot name="empty" />
		{/each}
	</tbody>
</table>

<style lang="scss">
	@import "../global_mixin";

	table {
		width: 100%;

		.elem:hover {
			background-color: $highlight-weak;
			cursor: pointer;
		}

		.selected {
			background-color: $highlight-strong;
		}
	}

	tr,
	th,
	td {
		@extend %unselectable;
	}

	.rowSelect {
		display: none;

		&:checked + tr {
			background-color: $highlight-strong;
		}
	}

	.isSortable {
		cursor: pointer;
	}
</style>
