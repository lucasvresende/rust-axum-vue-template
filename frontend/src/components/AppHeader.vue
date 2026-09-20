<script setup lang="ts">
import { computed, ref } from "vue";
import Button from "primevue/button";
import Popover from "primevue/popover";
import type { User } from "../types";

const props = defineProps<{ user: User; sidebarVisible: boolean }>();
const dark = defineModel<boolean>("dark", { required: true });
const emit = defineEmits<{ logout: []; toggleMenu: []; home: [] }>();
const account = ref<InstanceType<typeof Popover> | null>(null);
const accountVisible = ref(false);
const initials = computed(() => props.user.full_name?.slice(0, 1).toUpperCase() || "U");
</script>

<template>
    <header
        class="sticky top-0 z-20 flex h-20 items-center justify-between border-b border-surface bg-surface-0 px-4 dark:bg-surface-900 lg:px-8"
    >
        <div class="flex items-center gap-3 lg:gap-6">
            <button
                type="button"
                aria-label="Go to overview"
                class="flex items-center gap-2 rounded-lg text-lg font-semibold tracking-tight focus-visible:outline-2 focus-visible:outline-offset-4 focus-visible:outline-primary sm:text-2xl"
                @click="emit('home')"
            >
                <i class="pi pi-bolt text-primary" aria-hidden="true"></i> Axum + Vue
            </button>
            <Button
                icon="pi pi-bars"
                text
                rounded
                severity="secondary"
                aria-label="Toggle navigation"
                aria-controls="workspace-navigation"
                :aria-expanded="sidebarVisible"
                @click="emit('toggleMenu')"
            />
        </div>
        <div class="flex items-center gap-1 sm:gap-3">
            <Button
                :icon="dark ? 'pi pi-sun' : 'pi pi-moon'"
                text
                rounded
                aria-label="Toggle dark mode"
                @click="dark = !dark"
            />
            <button
                type="button"
                aria-label="Account"
                aria-haspopup="dialog"
                aria-controls="account-popover"
                :aria-expanded="accountVisible"
                class="grid size-9 shrink-0 place-items-center rounded-full bg-primary/10 font-semibold text-primary transition-colors hover:bg-primary/20 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-primary"
                @click="account?.toggle($event)"
            >
                {{ initials }}
            </button>
            <Popover
                ref="account"
                id="account-popover"
                aria-label="Account details"
                class="max-w-[calc(100vw-2rem)]"
                @show="accountVisible = true"
                @hide="accountVisible = false"
            >
                <div class="grid min-w-48 gap-3">
                    <div>
                        <p class="font-semibold">
                            {{ user.full_name || "Your account" }}
                        </p>
                        <p class="break-all text-sm text-muted-color">
                            {{ user.email }}
                        </p>
                        <p class="mt-2 text-sm text-primary">
                            {{ user.is_superuser ? "Admin" : "Member" }}
                        </p>
                    </div>
                    <Button
                        label="Sign out"
                        icon="pi pi-sign-out"
                        severity="secondary"
                        outlined
                        @click="emit('logout')"
                    />
                </div>
            </Popover>
            <Button
                label="Sign out"
                class="hidden sm:inline-flex"
                severity="secondary"
                text
                @click="emit('logout')"
            />
        </div>
    </header>
</template>
