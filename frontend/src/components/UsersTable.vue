<script setup lang="ts">
import { computed } from "vue";
import Card from "primevue/card";
import WorkspaceTable from "./WorkspaceTable.vue";
import type { User, TableColumn } from "../types";

const props = defineProps<{ users: User[] }>();
const indexedRows = computed(() =>
  props.users.map((row, index) => ({
    ...row,
    rowIndex: index + 1,
    role: row.is_superuser ? "Admin" : "Member",
  })),
);
const columns: TableColumn[] = [
  { field: "rowIndex", header: "#", filterType: "number" },
  { field: "full_name", header: "Name" },
  { field: "email", header: "Email" },
  { field: "role", header: "Role" },
];
</script>

<template>
  <Card class="overflow-hidden rounded-2xl border border-surface shadow-none">
    <template #title>Users</template>
    <template #content>
      <WorkspaceTable
        :rows="indexedRows"
        :columns="columns"
        name="users"
        empty="No workspace members found."
      />
    </template>
  </Card>
</template>
