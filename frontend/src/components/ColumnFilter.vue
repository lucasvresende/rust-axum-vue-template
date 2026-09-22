<script setup lang="ts">
import { computed } from "vue";
import { FilterMatchMode } from "@primevue/core/api";
import type { DataTableFilterMetaData } from "primevue/datatable";
import Select from "primevue/select";
import MultiSelect from "primevue/multiselect";
import InputText from "primevue/inputtext";
import InputNumber from "primevue/inputnumber";
import DatePicker from "primevue/datepicker";
import type { TableColumn } from "../types";

const props = defineProps<{
  column: TableColumn;
  filterModel: DataTableFilterMetaData;
  rows: Record<string, unknown>[];
}>();
const modes = computed(() => [
  props.column.filterType === "number" || props.column.filterType === "date"
    ? { label: "Range", value: FilterMatchMode.BETWEEN }
    : { label: "Search", value: FilterMatchMode.CONTAINS },
  { label: "Selected values", value: FilterMatchMode.IN },
]);
const options = computed(() => {
  const field = props.column.filterField || props.column.field;
  const values = [...new Set(props.rows.map((row) => row[field]))].filter(
    (value) => value !== null && value !== undefined,
  );
  return values
    .sort((a, b) =>
      typeof a === "number" && typeof b === "number"
        ? a - b
        : String(a).localeCompare(String(b)),
    )
    .map((value) => ({
      value,
      label:
        props.column.filterType === "date"
          ? new Intl.DateTimeFormat(undefined, { dateStyle: "medium" }).format(
              new Date(value as number),
            )
          : value === ""
            ? "(Blank)"
            : String(value),
    }));
});
function changeMode(mode: string) {
  props.filterModel.matchMode = mode;
  props.filterModel.value = null;
}
function bound(index: number): number | null {
  const value = props.filterModel.value?.[index];
  return Number.isFinite(value) ? value : null;
}
function dateBound(index: number) {
  const value = bound(index);
  return value === null ? null : new Date(value);
}
function updateBound(index: number, value: number | Date | null | undefined) {
  const range = [...(props.filterModel.value || [-Infinity, Infinity])];
  range[index] =
    value instanceof Date
      ? value.getTime()
      : (value ?? (index === 0 ? -Infinity : Infinity));
  props.filterModel.value = range.every((value) => !Number.isFinite(value))
    ? null
    : range;
}
</script>

<template>
  <div class="flex w-60 max-w-full min-w-0 flex-col gap-3">
    <Select
      appendTo="self"
      class="w-full min-w-0"
      :modelValue="filterModel.matchMode"
      :options="modes"
      optionLabel="label"
      optionValue="value"
      :aria-label="`${column.header} filter type`"
      @update:modelValue="changeMode"
    />
    <InputText
      v-if="filterModel.matchMode === FilterMatchMode.CONTAINS"
      class="w-full min-w-0"
      v-model="filterModel.value"
      :aria-label="`Search ${column.header}`"
      placeholder="Contains…"
    />
    <MultiSelect
      appendTo="self"
      v-else-if="filterModel.matchMode === FilterMatchMode.IN"
      class="w-full min-w-0"
      v-model="filterModel.value"
      :options="options"
      optionLabel="label"
      optionValue="value"
      filter
      :pt="{
        pcFilter: { root: { 'aria-label': `Find ${column.header} values` } },
      }"
      :aria-label="`Select ${column.header} values`"
      placeholder="Any value"
      :maxSelectedLabels="2"
      selectedItemsLabel="{0} values selected"
    />
    <template v-else-if="column.filterType === 'date'">
      <DatePicker
        :showOnFocus="false"
        class="w-full min-w-0"
        inputClass="w-full min-w-0"
        :modelValue="dateBound(0)"
        :aria-label="`${column.header} from`"
        placeholder="From date"
        dateFormat="yy-mm-dd"
        showIcon
        :maxDate="dateBound(1) || undefined"
        @update:modelValue="updateBound(0, $event as Date | null)"
      />
      <DatePicker
        :showOnFocus="false"
        class="w-full min-w-0"
        inputClass="w-full min-w-0"
        :modelValue="dateBound(1)"
        :aria-label="`${column.header} to`"
        placeholder="To date"
        dateFormat="yy-mm-dd"
        showIcon
        :minDate="dateBound(0) || undefined"
        @update:modelValue="updateBound(1, $event as Date | null)"
      />
    </template>
    <template v-else>
      <InputNumber
        class="w-full min-w-0"
        inputClass="w-full min-w-0"
        :modelValue="bound(0)"
        :aria-label="`${column.header} minimum`"
        placeholder="Minimum"
        :max="bound(1) ?? undefined"
        @update:modelValue="updateBound(0, $event)"
      />
      <InputNumber
        class="w-full min-w-0"
        inputClass="w-full min-w-0"
        :modelValue="bound(1)"
        :aria-label="`${column.header} maximum`"
        placeholder="Maximum"
        :min="bound(0) ?? undefined"
        @update:modelValue="updateBound(1, $event)"
      />
    </template>
  </div>
</template>
