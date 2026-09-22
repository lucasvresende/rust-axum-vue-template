<script setup lang="ts">
import Button from "primevue/button";
import type { User, WorkspaceView } from "../types";

defineProps<{ user: User; activeView: WorkspaceView }>();
const emit = defineEmits<{ create: [] }>();
</script>

<template>
  <div class="flex flex-col justify-between gap-4 sm:flex-row sm:items-center">
    <div v-if="activeView === 'overview'">
      <p
        class="mb-2 text-xs font-semibold uppercase tracking-widest text-muted-color"
      >
        DASHBOARD
      </p>
      <h1 class="mb-2 text-2xl font-semibold tracking-tight lg:text-3xl">
        Good to see you, {{ user.full_name || user.email }}.
      </h1>
      <p class="text-muted-color">
        Manage your account and your personal inventory.
      </p>
    </div>
    <div v-else>
      <p
        class="mb-2 text-xs font-semibold uppercase tracking-widest text-muted-color"
      >
        WORKSPACE
      </p>
      <h1 class="mb-2 text-2xl font-semibold tracking-tight lg:text-3xl">
        {{ activeView === "items" ? "My items" : "Users" }}
      </h1>
      <p class="text-muted-color">
        {{
          activeView === "items"
            ? "Manage your personal inventory."
            : "View workspace members and their roles."
        }}
      </p>
    </div>
    <Button
      v-if="activeView !== 'users'"
      label="New item"
      icon="pi pi-plus"
      @click="emit('create')"
    />
  </div>
</template>
