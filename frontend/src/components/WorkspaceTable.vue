<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { FilterMatchMode } from "@primevue/core/api";
import DataTable from "primevue/datatable";
import Column from "primevue/column";
import Button from "primevue/button";
import InputText from "primevue/inputtext";
import MultiSelect from "primevue/multiselect";
import ColumnFilter from "./ColumnFilter.vue";
import type { TableColumn } from "../types";

const props = defineProps<{
  rows: Record<string, unknown>[];
  columns: TableColumn[];
  name: string;
  empty: string;
}>();
const selected = ref(props.columns.map((column) => column.field));
const visibleColumns = computed(() =>
  props.columns.filter((column) => selected.value.includes(column.field)),
);
const first = ref(0);
const initialFilters = () =>
  Object.fromEntries([
    ["global", { value: null, matchMode: FilterMatchMode.CONTAINS }],
    ...props.columns.map((column) => [
      column.filterField || column.field,
      {
        value: null,
        matchMode:
          column.filterType === "number" || column.filterType === "date"
            ? FilterMatchMode.BETWEEN
            : FilterMatchMode.CONTAINS,
      },
    ]),
  ]);
const filters = ref(initialFilters());
function clearFilters() {
  filters.value = initialFilters();
  first.value = 0;
}
watch(selected, () => {
  for (const column of props.columns) {
    if (!selected.value.includes(column.field))
      filters.value[column.filterField || column.field].value = null;
  }
  first.value = 0;
});
</script>

<template>
  <DataTable
    v-model:first="first"
    v-model:filters="filters"
    :value="rows"
    :globalFilterFields="columns.map((column) => column.field)"
    filterDisplay="menu"
    paginator
    :rows="10"
    :rowsPerPageOptions="[5, 10, 20]"
    dataKey="id"
    scrollable
  >
    <template #header>
      <div class="flex flex-wrap items-center gap-3">
        <Button
          label="Clear all filters"
          icon="pi pi-filter-slash"
          severity="secondary"
          outlined
          @click="clearFilters"
        />
        <MultiSelect
          v-model="selected"
          :options="columns"
          optionLabel="header"
          optionValue="field"
          :aria-label="`${name} columns`"
          placeholder="Columns"
          :maxSelectedLabels="0"
          selectedItemsLabel="{0} columns"
          class="w-full sm:w-48"
        />
        <InputText
          v-model="filters.global.value"
          :aria-label="`Search ${name}`"
          placeholder="Search table…"
          class="w-full sm:ml-auto sm:w-60"
        />
      </div>
      <p class="mt-2 text-xs text-muted-color sm:hidden">
        Scroll sideways to see all columns.
      </p>
    </template>
    <Column
      v-for="column in visibleColumns"
      :key="column.field"
      :field="column.field"
      :sortField="column.sortField || column.field"
      :header="column.header"
      sortable
      :filterField="column.filterField || column.field"
      :showFilterMatchModes="false"
      :showFilterOperator="false"
      :showAddButton="false"
      :pt="{
        headerCell: { 'aria-label': column.header },
        pcColumnFilterButton: { 'aria-label': `Filter ${column.header}` },
        filterOverlay: { 'aria-label': `Filter ${column.header}` },
      }"
      :style="{
        minWidth:
          column.field === 'rowIndex'
            ? '6rem'
            : column.filterType === 'date'
              ? '13rem'
              : '8rem',
      }"
    >
      <template #body="{ data }">
        <slot :name="column.field" :data="data">{{ data[column.field] }}</slot>
      </template>
      <template #filter="{ filterModel }">
        <ColumnFilter
          :column="column"
          :filterModel="filterModel"
          :rows="rows"
        />
      </template>
    </Column>
    <Column
      v-if="$slots.actions"
      header="Actions"
      style="width: 5rem; min-width: 5rem"
      alignFrozen="right"
      frozen
    >
      <template #body="{ data }"><slot name="actions" :data="data" /></template>
    </Column>
    <template #empty>{{
      rows.length ? "No results match your filters." : empty
    }}</template>
  </DataTable>
</template>
