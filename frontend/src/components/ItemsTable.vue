<script setup lang="ts">
import { computed, ref } from "vue";
import ItemActions from "./ItemActions.vue";
import Card from "primevue/card";
import DataTable from "primevue/datatable";
import Column from "primevue/column";
import type { Item } from "../types";

const props = defineProps<{ items: Item[] }>();
const indexedRows = computed(() =>
  props.items.map((row, index) => ({ ...row, rowIndex: index + 1 })),
);
const emit = defineEmits<{ remove: [item: Item]; edit: [item: Item] }>();
const first = ref(0);
const dateFormat = new Intl.DateTimeFormat(undefined, {
  dateStyle: "medium",
  timeStyle: "short",
});

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
      <DataTable
        v-model:first="first"
        :value="indexedRows"
        paginator
        :rows="10"
        :rowsPerPageOptions="[5, 10, 20]"
        responsiveLayout="scroll"
        dataKey="id"
      >
        <Column field="rowIndex" header="#" class="w-16" sortable />
        <Column field="title" header="Title" sortable />
        <Column field="description" header="Description" />
        <Column field="quantity" header="Quantity" sortable />
        <Column field="created_at" header="Creation date" sortable>
          <template #body="{ data }">
            <time :datetime="data.created_at" class="whitespace-nowrap">{{
              formatDate(data.created_at)
            }}</time>
          </template>
        </Column>
        <Column header="Actions">
          <template #body="{ data }">
            <ItemActions
              :item="data"
              @edit="emit('edit', $event)"
              @remove="emit('remove', $event)"
            />
          </template>
        </Column>
        <template #empty> No items yet. Create your first one. </template>
      </DataTable>
    </template>
  </Card>
</template>
