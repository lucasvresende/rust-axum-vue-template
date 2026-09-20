<script setup lang="ts">
import { RouterLink } from "vue-router";
import type { WorkspaceView } from "../types";

defineProps<{ isSuperuser: boolean; activeView: WorkspaceView }>();
</script>

<template>
    <aside
        id="workspace-navigation"
        class="shrink-0 rounded-2xl border border-surface bg-surface-0 p-5 dark:bg-surface-900 lg:sticky lg:top-28 lg:h-[calc(100vh-9rem)] lg:w-64"
    >
        <p
            class="mb-3 px-3 text-xs font-bold uppercase tracking-wider text-muted-color"
        >
            Workspace
        </p>
        <nav aria-label="Workspace" class="flex gap-1 lg:flex-col">
            <template
                v-for="entry in [
                    { id: 'overview', label: 'Overview', icon: 'pi-home' },
                    { id: 'items', label: 'My items', icon: 'pi-box' },
                    { id: 'users', label: 'Users', icon: 'pi-users' },
                ] as const"
                :key="entry.id"
            >
                <RouterLink
                    v-if="entry.id !== 'users' || isSuperuser"
                    :to="{ name: entry.id }"
                    class="flex flex-1 items-center gap-2 whitespace-nowrap rounded-lg px-3 py-3 text-left text-sm transition-colors hover:bg-emphasis focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-primary lg:flex-none"
                    :class="
                        activeView === entry.id
                            ? 'bg-highlight font-semibold text-primary'
                            : 'text-muted-color'
                    "
                    :aria-current="activeView === entry.id ? 'page' : undefined"
                >
                    <i :class="['pi', entry.icon]" aria-hidden="true"></i>
                    {{ entry.label }}
                </RouterLink>
            </template>
        </nav>
    </aside>
</template>
