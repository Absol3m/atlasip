<script lang="ts">
  import type { IpRecord, ColumnDef } from '$lib/types/ip';

  interface Props {
    record: IpRecord;
    columns: ColumnDef[];
    isSelected: boolean;
    isEven: boolean;
    onrowclick: (record: IpRecord, e: MouseEvent) => void;
    onrowdblclick: (record: IpRecord) => void;
    onrowcontextmenu: (record: IpRecord, e: MouseEvent) => void;
    oncheckboxclick: (record: IpRecord, e: MouseEvent) => void;
  }

  let {
    record,
    columns,
    isSelected,
    isEven,
    onrowclick,
    onrowdblclick,
    onrowcontextmenu,
    oncheckboxclick,
  }: Props = $props();
</script>

<tr
  class:selected={isSelected}
  class:even={isEven}
  onclick={(e) => onrowclick(record, e)}
  ondblclick={() => onrowdblclick(record)}
  oncontextmenu={(e) => { e.preventDefault(); onrowcontextmenu(record, e); }}
>
  <td class="col-check">
    <input
      type="checkbox"
      checked={isSelected}
      onclick={(e) => { e.stopPropagation(); oncheckboxclick(record, e); }}
    />
  </td>
  {#each columns as col (col.key)}
    <td
      class:sticky-ip={col.key === 'ip'}
      title={col.getValue(record)}
      data-colkey={col.key}
    >
      <span class="cell-text">{col.getValue(record)}</span>
    </td>
  {/each}
</tr>

<style>
  tr {
    cursor: pointer;
    background: var(--color-surface);
    transition: background-color 0.1s;
  }

  tr.even {
    background: var(--color-row-even);
  }

  tr:hover,
  tr.even:hover {
    background: var(--color-row-hover);
  }

  tr.selected,
  tr.even.selected {
    background: var(--color-row-selected);
  }

  td {
    padding: 0 12px;
    height: 36px;
    border-bottom: 1px solid var(--color-border);
    white-space: nowrap;
    font-size: 13px;
    color: var(--color-text);
    vertical-align: middle;
    max-width: 0;
  }

  td.col-check {
    width: 40px;
    min-width: 40px;
    max-width: 40px;
    text-align: center;
    padding: 0;
    position: sticky;
    left: 0;
    z-index: 1;
    background: inherit;
  }

  td.sticky-ip {
    position: sticky;
    left: 40px;
    z-index: 1;
    background: inherit;
    font-weight: 500;
    font-family: 'JetBrains Mono', 'Fira Code', ui-monospace, monospace;
    font-size: 12.5px;
    color: var(--color-accent);
  }

  .cell-text {
    display: block;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  input[type='checkbox'] {
    width: 15px;
    height: 15px;
    cursor: pointer;
    accent-color: var(--color-accent);
    display: block;
    margin: auto;
  }
</style>
