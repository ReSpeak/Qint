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
<script lang="ts">
	import { createEventDispatcher } from "svelte";
	import { SortOrder } from "./uiTable";
	import type {
		ColumnKey,
		IColumn,
		IColumns,
		IRows,
		IRowOptions,
		ClickRowData,
		IDragOptions,
	} from "./uiTable";
	import { draggable, DragData } from "../util/draggable";
	import Icon from "../icon/Icon.svelte";

	const dispatch = createEventDispatcher<{
		clickCol: { event: MouseEvent; col: TCol; key: ColumnKey };
		clickRow: ClickRowData<TRow>;
		clickCell: { event: MouseEvent; row: TRow; key: ColumnKey };
		selectionChanged: { selected: TRow[] };
		dragEnter: { target: HTMLElement };
		dragLeave: { target: HTMLElement };
		dragDrop: { target: HTMLElement };
	}>();

	type TRow = $$Generic;
	type TCol = IColumn<TRow>;
	type InternalRow = {
		t: TRow;
		id: number;
		selected: boolean;
		sortVal?: any;
	};
	interface $$Slots {
		headerCell: { col: TCol };
		orderIcon: { sortOrder: SortOrder };
		colCell: { col: TCol; row: TRow };
	}

	export let columns: IColumns<TRow>;
	export let rows: IRows<TRow>;
	export let rowOptions: IRowOptions<TRow> = {};
	export let dragOptions: IDragOptions = {};
	export let sortBy: ColumnKey = "";
	export let sortOrder: SortOrder = SortOrder.Asc;

	let c_rows: InternalRow[];
	let columnByKey: Record<ColumnKey, TCol> = {};

	export function clearSelection(): void {
		clearSelectionInternal(true);
	}

	$: {
		columnByKey = {};
		columns.forEach((col) => {
			columnByKey[col.key] = col;
		});
	}

	$: remap(rows);

	function remap(_rows: IRows<TRow>) {
		clearSelectionInternal(true);
		c_rows = _rows.map((r, id) => {
			return { t: r, selected: false, id };
		});
		// c_rows = c_rows.filter((r) => {
		// })
	}

	$: reSort(), sortBy, sortOrder, rows;

	function reSort() {
		clearSelectionInternal(true);
		if (sortBy === "") return;
		const sortFn = columnByKey[sortBy].sort;
		if (sortFn === undefined) return;
		c_rows = c_rows.sort((a, b) => sortFn!(a.t, b.t, sortOrder));
		c_rows.forEach((r, i) => {
			r.id = i;
		});
	}

	function updateSortOrder(colKey: ColumnKey) {
		if (colKey === sortBy) {
			sortOrder = sortOrder === SortOrder.Asc ? SortOrder.Desc : SortOrder.Asc;
		} else {
			sortOrder = SortOrder.Asc;
		}
	}

	const selected = new Set<number>();
	let lastSelected: number = 0;

	function handleClickCol(event: MouseEvent, col: TCol) {
		if (col.sort === undefined) return;
		updateSortOrder(col.key);
		sortBy = col.key;
		dispatch("clickCol", { event, col, key: col.key });
	}

	function clearSelectionInternal(triggerEvent: boolean): boolean {
		let hasChanged = false;
		for (const oldSel of selected.values()) {
			c_rows[oldSel].selected = false;
			hasChanged = true;
		}
		selected.clear();
		if (hasChanged && triggerEvent) selectionChanged();
		return hasChanged;
	}

	function unselectElem(row: InternalRow) {
		if (selected.has(row.id)) {
			selected.delete(row.id);
			row.selected = false;
			selectionChanged();
		}
	}

	function selectElem(add: boolean, ...selectRows: InternalRow[]) {
		let hasChanged = false;
		if (!add) {
			hasChanged ||= clearSelectionInternal(false);
		}
		for (const row of selectRows) {
			if (!selected.has(row.id)) {
				selected.add(row.id);
				row.selected = true;
				hasChanged = true;
			}
		}
		if (hasChanged) selectionChanged();
	}

	function toggleElem(add: boolean, row: InternalRow) {
		if (selected.has(row.id)) {
			unselectElem(row);
		} else {
			selectElem(add, row);
		}
	}

	function selectionChanged() {
		c_rows = c_rows;
		const newSel = Array.from(selected.values(), (id) => c_rows[id].t);
		dispatch("selectionChanged", { selected: newSel });
	}

	function handleClickRow(event: MouseEvent, row: InternalRow, dblclick: boolean) {
		const isRealDblClick = dblclick && !event.ctrlKey && !event.shiftKey;
		if (!dblclick) {
			if (event.ctrlKey) {
				lastSelected = row.id;
				toggleElem(true, row);
				event.preventDefault();
			} else if (event.shiftKey) {
				const [start, end] =
					row.id > lastSelected ? [lastSelected, row.id] : [row.id, lastSelected];
				selectElem(false, ...c_rows.slice(start, end + 1));
			} else {
				lastSelected = row.id;
				if (selected.has(row.id) && selected.size === 1) {
					unselectElem(row);
				} else {
					selectElem(false, row);
				}
			}
		}

		dispatch("clickRow", { event, row: row.t, dblclick: isRealDblClick });
	}

	function handleClickCell(event: MouseEvent, row: InternalRow, key: ColumnKey) {
		dispatch("clickCell", { event, row: row.t, key });
	}

	let draggingElements = false;
	let dragVisualizer: HTMLElement;
	let dropTargets: HTMLElement[] = [];
	let lastDropTarget: HTMLElement | undefined = undefined;

	function dragEnter(e: MouseEvent) {
		if (!draggingElements) return;
		lastDropTarget = e.target as HTMLElement;
		dispatch("dragEnter", { target: lastDropTarget });
	}

	function dragLeave(e: MouseEvent) {
		if (!draggingElements) return;
		lastDropTarget = undefined;
		dispatch("dragLeave", { target: e.target as HTMLElement });
	}

	function dragStart(ev: CustomEvent<DragData>, row: InternalRow) {
		if (!row.selected) {
			selectElem(false, row);
		}
		draggingElements = true;
		dropTargets = dragOptions.dropFilter ? dragOptions.dropFilter() : [];
		for (const dropTarget of dropTargets) {
			dropTarget.addEventListener("mouseenter", dragEnter);
			dropTarget.addEventListener("mouseleave", dragLeave);
		}

		dragVisualizer.style.display = null!;
		ev.detail.dragNode = dragVisualizer;
		const rect = dragVisualizer.getBoundingClientRect();
		const dx = ev.detail.mouseStart.clientX - rect.x;
		const dy = ev.detail.mouseStart.clientY - rect.y;
		ev.detail.x -= dx;
		ev.detail.y -= dy;
		dragVisualizer.style.transform = `translate(${dx}px,${dy}px)`;
	}

	function dragDrop(_ev: CustomEvent<DragData>) {
		for (const dropTarget of dropTargets) {
			dropTarget.removeEventListener("mouseenter", dragEnter);
			dropTarget.removeEventListener("mouseleave", dragLeave);
		}
		dropTargets = [];
		draggingElements = false;
		dragVisualizer.style.display = "none";

		if (lastDropTarget !== undefined) {
			dispatch("dragDrop", { target: lastDropTarget });
			lastDropTarget = undefined;
		}
	}
</script>

<div class="dragVisualize" bind:this={dragVisualizer} style="display: none;">
	<Icon name="file-multiple-outline" />
</div>
<div class="scrollContainer">
	<table on:click|stopPropagation class="table" class:draggingElements>
		<thead>
			<tr>
				{#each columns as col}
					<th
						on:click={(e) => handleClickCol(e, col)}
						class:isSortable={col.sort !== undefined}
						class={col.headerClass}>
						{#if col.customRender === true}
							<slot name="headerCell" {col} />
						{:else}{col.title}{/if}
						{#if sortBy === col.key}
							<slot name="orderIcon" {sortOrder}
								>{sortOrder === SortOrder.Asc ? "▲" : "▼"}</slot>
						{/if}
					</th>
				{/each}
			</tr>
		</thead>
		<tbody>
			<slot />
			{#each c_rows as row}
				<tr
					use:draggable={true}
					on:svddrag={(e) => dragStart(e, row)}
					on:svddrop={dragDrop}
					on:click={(e) => handleClickRow(e, row, false)}
					on:dblclick={(e) => handleClickRow(e, row, true)}
					class:selected={row.selected}
					data-type={rowOptions.dataType ? rowOptions.dataType(row.t) : null}
					data-key={rowOptions.dataValue ? rowOptions.dataValue(row.t) : null}>
					{#each columns as col}
						<td
							on:click={(e) => {
								handleClickCell(e, row, col.key);
							}}
							class={col.class}>
							{#if col.customRender === true}
								<slot name="colCell" {col} row={row.t} />
							{:else}
								{col.renderValue ? col.renderValue(row.t) : col.value(row.t)}
							{/if}
						</td>
					{/each}
				</tr>
			{:else}
				<slot name="empty" />
			{/each}
		</tbody>
	</table>
</div>

<style lang="scss">
	@import "../../style/global_mixin";

	table {
		position: relative;
		width: 100%;

		.elem:hover {
			background-color: $highlight-weak;
			cursor: pointer;
		}

		.selected {
			background-color: $highlight-strong;
		}

		&.draggingElements .selected {
			background-color: $highlight-weak;
			color: darken($text, 50%);
		}
	}

	thead {
		position: sticky;
		top: 0;
		background-color: inherit;
	}

	tr,
	th,
	td {
		@extend %unselectable;
	}

	.scrollContainer {
		overflow-x: hidden;
		overflow-y: scroll;
	}

	.isSortable {
		cursor: pointer;
	}

	.dragVisualize {
		display: flex;
		justify-content: center;
		align-items: center;
		background-color: rgba(20, 70, 70, 0.6);
		width: 5rem;
		height: 5rem;
		font-size: 3em;
		line-height: 1em;
		border-radius: 0.5em;
		border: #1e5050 solid 1px;
		box-shadow: 5px 5px 10px 5px rgba(30, 30, 30, 0.5);
		position: absolute;
		z-index: 200;
		pointer-events: none;
	}
</style>
