<script setup lang="ts">

defineProps<{ itemCount: number; isSuperuser: boolean }>();
</script>

<template>
    <div class="grid gap-6 md:grid-cols-3">
        <div v-for="stat in [
            { label: 'My items', value: itemCount, icon: 'pi-box', detail: 'In your personal inventory', color: 'bg-blue-100 text-blue-600 dark:bg-blue-400/10 dark:text-blue-400' },
            { label: 'Account', value: isSuperuser ? 'Admin' : 'Member', icon: 'pi-user', detail: 'Your workspace role', color: 'bg-orange-100 text-orange-600 dark:bg-orange-400/10 dark:text-orange-400' },
            { label: 'Session', value: 'Signed in', icon: 'pi-shield', detail: 'Authenticated workspace access', color: 'bg-emerald-100 text-emerald-600 dark:bg-emerald-400/10 dark:text-emerald-400' }
        ]" :key="stat.label" class="rounded-2xl border border-surface bg-surface-0 p-6 dark:bg-surface-900">
            <div class="mb-4 flex items-start justify-between gap-3">
                <div><p class="mb-3 font-medium text-muted-color">{{ stat.label }}</p><p class="text-2xl font-semibold">{{ stat.value }}</p></div>
                <span :class="stat.color" class="grid size-11 shrink-0 place-items-center rounded-lg"><i :class="['pi', stat.icon]" aria-hidden="true"></i></span>
            </div>
            <p class="text-sm text-muted-color">{{ stat.detail }}</p>
        </div>
    </div>
</template>
