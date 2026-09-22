<script setup lang="ts">
import { computed } from "vue";
import ItemActions from "./ItemActions.vue";
import Card from "primevue/card";
import WorkspaceTable from "./WorkspaceTable.vue";
import type { Item, TableColumn } from "../types";

const props = defineProps<{ items: Item[] }>();
const indexedRows = computed(() =>
  props.items.map((row, index) => ({
    ...row,
    rowIndex: index + 1,
    creationDate: formatDate(row.created_at),
    creationDay: calendarDay(row.created_at),
  })),
);
const emit = defineEmits<{ remove: [item: Item]; edit: [item: Item] }>();
const columns: TableColumn[] = [
  { field: "rowIndex", header: "#", filterType: "number" },
  { field: "title", header: "Title" },
  { field: "description", header: "Description" },
  { field: "quantity", header: "Quantity", filterType: "number" },
  {
    field: "creationDate",
    header: "Creation date",
    sortField: "created_at",
    filterField: "creationDay",
    filterType: "date",
  },
];
const dateFormat = new Intl.DateTimeFormat(undefined, {
  dateStyle: "medium",
  timeStyle: "short",
});

function calendarDay(value: string) {
  const date = new Date(value);
  return Number.isNaN(date.getTime())
    ? null
    : new Date(date.getFullYear(), date.getMonth(), date.getDate()).getTime();
}

/** Format server timestamps in the browser’s locale, with a fallback for invalid dates. */
function formatDate(value: string) {
  const date = new Date(value);
  return Number.isNaN(date.getTime()) ? "—" : dateFormat.format(date);
}
</script>

<template>
  <Card class="overflow-hidden rounded-2xl border border-surface shadow-none">
    <template #title>My items</template>
    <template #content>
      <WorkspaceTable
        :rows="indexedRows"
        :columns="columns"
        name="items"
        empty="No items yet. Create your first one."
      >
        <template #creationDate="{ data }">
          <time :datetime="data.created_at" class="whitespace-nowrap">{{
            data.creationDate
          }}</time>
        </template>
        <template #actions="{ data }">
          <ItemActions
            :item="data"
            @edit="emit('edit', $event)"
            @remove="emit('remove', $event)"
          />
        </template>
      </WorkspaceTable>
    </template>
  </Card>
</template>
