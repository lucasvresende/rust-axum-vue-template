<script setup lang="ts">
import { computed } from "vue";
import Button from "primevue/button";
import type { User } from "../types";

const props = defineProps<{ user: User }>();
const dark = defineModel<boolean>("dark", { required: true });
const emit = defineEmits<{ logout: [] }>();
const initials = computed(() => props.user.full_name?.slice(0, 1).toUpperCase() || "U");
</script>

<template>
    <header>
        <div class="brand"><i class="pi pi-bolt"></i> Axum + Vue</div>
        <div class="actions">
            <Button
                :icon="dark ? 'pi pi-sun' : 'pi pi-moon'"
                text
                rounded
                aria-label="Toggle dark mode"
                @click="dark = !dark"
            />
            <span class="avatar">{{ initials }}</span>
            <Button
                label="Sign out"
                severity="secondary"
                text
                @click="emit('logout')"
            />
        </div>
    </header>
</template>
