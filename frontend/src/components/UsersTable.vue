<script setup lang="ts">
import { computed, ref } from "vue";
import Button from "primevue/button";
import Card from "primevue/card";
import Dialog from "primevue/dialog";
import InputText from "primevue/inputtext";
import WorkspaceTable from "./WorkspaceTable.vue";
import { api } from "../api";
import type { User, TableColumn } from "../types";

const props = defineProps<{ users: User[]; currentUserId?: string }>();
const emit = defineEmits<{ changed: [] }>();
const indexedRows = computed(() =>
  props.users.map((row, index) => ({
    ...row,
    rowIndex: index + 1,
    role: row.is_superuser ? "Admin" : "Member",
    status: row.is_active
      ? row.needs_setup
        ? "Invited"
        : "Active"
      : "Inactive",
  })),
);
const columns: TableColumn[] = [
  { field: "rowIndex", header: "#", filterType: "number" },
  { field: "full_name", header: "Name" },
  { field: "email", header: "Email" },
  { field: "role", header: "Role" },
  { field: "status", header: "Status" },
];
const dialog = ref(false);
const name = ref("");
const email = ref("");
const busy = ref(false);
const error = ref("");
async function create() {
  busy.value = true;
  error.value = "";
  try {
    await api("/users", {
      method: "POST",
      body: JSON.stringify({ email: email.value, full_name: name.value }),
    });
    dialog.value = false;
    name.value = "";
    email.value = "";
    emit("changed");
  } catch (e) {
    error.value = e instanceof Error ? e.message : "Request failed.";
  } finally {
    busy.value = false;
  }
}
async function status(row: User) {
  if (busy.value) return;
  busy.value = true;
  error.value = "";
  try {
    await api(`/users/${row.id}/status`, {
      method: "PUT",
      body: JSON.stringify({ is_active: !row.is_active }),
    });
    emit("changed");
  } catch (e) {
    error.value = e instanceof Error ? e.message : "Request failed.";
  } finally {
    busy.value = false;
  }
}
async function resend(row: User) {
  if (busy.value) return;
  busy.value = true;
  error.value = "";
  try {
    await api(`/users/${row.id}/resend-invitation`, { method: "POST" });
    emit("changed");
  } catch (e) {
    error.value = e instanceof Error ? e.message : "Request failed.";
  } finally {
    busy.value = false;
  }
}
</script>
<template>
  <Card class="overflow-hidden rounded-2xl border border-surface shadow-none">
    <template #title
      ><div class="flex items-center justify-between gap-3">
        Users<Button
          label="New user"
          icon="pi pi-plus"
          @click="dialog = true"
        /></div
    ></template>
    <template #content>
      <p v-if="error" role="alert" class="mb-3 text-red-600">{{ error }}</p>
      <WorkspaceTable
        :rows="indexedRows"
        :columns="columns"
        name="users"
        empty="No workspace members found."
      >
        <template #actions="{ data }">
          <div class="flex gap-2">
            <Button
              v-if="data.needs_setup && data.is_active"
              label="Resend invite"
              text
              size="small"
              :disabled="busy"
              @click="resend(data)"
            />
            <Button
              :label="data.is_active ? 'Deactivate' : 'Activate'"
              :severity="data.is_active ? 'danger' : 'success'"
              text
              size="small"
              :disabled="busy || (data.id === currentUserId && data.is_active)"
              @click="status(data)"
            />
          </div>
        </template>
      </WorkspaceTable>
    </template>
  </Card>
  <Dialog
    v-model:visible="dialog"
    modal
    header="Create user"
    class="w-[calc(100vw-2rem)] max-w-md"
  >
    <form class="grid gap-4" @submit.prevent="create">
      <label class="grid gap-2"
        >Full name<InputText v-model="name" required maxlength="200"
      /></label>
      <label class="grid gap-2"
        >Email<InputText v-model="email" type="email" required
      /></label>
      <p class="text-sm text-muted-color">
        The user will receive an email link to set their own password.
      </p>
      <p v-if="error" role="alert" class="text-red-600">{{ error }}</p>
      <Button type="submit" label="Send invitation" :loading="busy" />
    </form>
  </Dialog>
</template>
