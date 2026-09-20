<script setup lang="ts">
import type { WorkspaceView } from "../types";

defineProps<{ isSuperuser: boolean }>();
const activeView = defineModel<WorkspaceView>({ required: true });
</script>

<template>
    <aside>
        <p class="overline">WORKSPACE</p>
        <nav aria-label="Workspace">
            <button
                type="button"
                :class="{ active: activeView === 'overview' }"
                :aria-current="
                    activeView === 'overview' ? 'page' : undefined
                "
                @click="activeView = 'overview'"
            >
                <i class="pi pi-home"></i> Overview
            </button>
            <button
                type="button"
                :class="{ active: activeView === 'items' }"
                :aria-current="activeView === 'items' ? 'page' : undefined"
                @click="activeView = 'items'"
            >
                <i class="pi pi-box"></i> My items
            </button>
            <button
                v-if="isSuperuser"
                type="button"
                :class="{ active: activeView === 'users' }"
                :aria-current="activeView === 'users' ? 'page' : undefined"
                @click="activeView = 'users'"
            >
                <i class="pi pi-users"></i> Users
            </button>
        </nav>
    </aside>
</template>
