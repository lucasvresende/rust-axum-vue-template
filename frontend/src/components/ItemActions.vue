<script setup lang="ts">
import { ref } from "vue";
import Button from "primevue/button";
import Menu from "primevue/menu";
import type { Item } from "../types";

const props = defineProps<{ item: Item }>();
const emit = defineEmits<{ edit: [item: Item]; remove: [item: Item] }>();
const menu = ref<InstanceType<typeof Menu> | null>(null);
const expanded = ref(false);
const actions = [
  {
    label: "Edit",
    icon: "pi pi-pencil",
    command: () => emit("edit", props.item),
  },
  {
    label: "Delete",
    icon: "pi pi-trash",
    command: () => emit("remove", props.item),
  },
];
</script>

<template>
  <Button
    label="Actions"
    icon="pi pi-chevron-down"
    iconPos="right"
    severity="secondary"
    outlined
    :aria-label="`Actions for ${item.title}`"
    aria-haspopup="menu"
    :aria-controls="`item-actions-${item.id}`"
    :aria-expanded="expanded"
    @click="menu?.toggle($event)"
  />
  <Menu
    ref="menu"
    :id="`item-actions-${item.id}`"
    :model="actions"
    popup
    @show="expanded = true"
    @hide="expanded = false"
  />
</template>
