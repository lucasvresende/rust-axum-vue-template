<script setup lang="ts">
import { computed } from "vue";
import Button from "primevue/button";
import type { User } from "../types";

const props = defineProps<{ user: User; sidebarVisible: boolean }>();
const dark = defineModel<boolean>("dark", { required: true });
const emit = defineEmits<{ logout: []; toggleMenu: [] }>();
const initials = computed(() => props.user.full_name?.slice(0, 1).toUpperCase() || "U");
</script>

<template>
    <header class="sticky top-0 z-20 flex h-20 items-center justify-between border-b border-surface bg-surface-0 px-4 dark:bg-surface-900 lg:px-8">
        <div class="flex items-center gap-3 lg:gap-6">
            <span class="flex items-center gap-2 text-lg font-semibold tracking-tight sm:text-2xl"><i class="pi pi-bolt text-primary" aria-hidden="true"></i> Axum + Vue</span>
            <Button icon="pi pi-bars" text rounded severity="secondary" aria-label="Toggle navigation" aria-controls="workspace-navigation" :aria-expanded="sidebarVisible" @click="emit('toggleMenu')" />
        </div>
        <div class="flex items-center gap-1 sm:gap-3">
            <Button
                :icon="dark ? 'pi pi-sun' : 'pi pi-moon'"
                text
                rounded
                aria-label="Toggle dark mode"
                @click="dark = !dark"
            />
            <span class="hidden size-9 place-items-center rounded-full bg-primary/10 font-semibold text-primary sm:grid">{{ initials }}</span>
            <Button
                label="Sign out"
                severity="secondary"
                text
                @click="emit('logout')"
            />
        </div>
    </header>
</template>
