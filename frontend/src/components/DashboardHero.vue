<script setup lang="ts">
import Button from "primevue/button";
import type { User, WorkspaceView } from "../types";

defineProps<{ user: User; activeView: WorkspaceView }>();
const emit = defineEmits<{ create: [] }>();
</script>

<template>
    <div class="hero">
        <div v-if="activeView === 'overview'">
            <p class="overline">DASHBOARD</p>
            <h1>
                Good to see you, {{ user.full_name || user.email }}.
            </h1>
            <p>Manage your account and your personal inventory.</p>
        </div>
        <div v-else>
            <p class="overline">WORKSPACE</p>
            <h1>{{ activeView === "items" ? "My items" : "Users" }}</h1>
            <p>
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
