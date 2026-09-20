<script setup lang="ts">
import Button from "primevue/button";
import Card from "primevue/card";
import InputText from "primevue/inputtext";
import Password from "primevue/password";

defineProps<{ loading: boolean }>();
const email = defineModel<string>("email", { required: true });
const password = defineModel<string>("password", { required: true });
const emit = defineEmits<{ submit: [] }>();
</script>

<template>
    <section class="grid min-h-screen place-items-center bg-surface-50 p-4 dark:bg-surface-950">
        <Card class="w-full max-w-md rounded-3xl border border-surface border-t-4 border-t-primary p-4 shadow-none sm:p-6">
            <template #title><i class="pi pi-bolt text-primary"></i> Axum + Vue</template>
            <template #subtitle>Welcome back. Sign in to your workspace.</template>
            <template #content>
                <form @submit.prevent="emit('submit')" class="mt-4 grid gap-5">
                    <label class="grid gap-2 text-sm font-medium">
                        Email<InputText
                            v-model="email"
                            type="email"
                            required
                            autocomplete="email"
                        />
                    </label>
                    <label class="grid gap-2 text-sm font-medium">
                        Password<Password
                            v-model="password"
                            :feedback="false"
                            fluid
                            required
                            toggleMask
                            autocomplete="current-password"
                        />
                    </label>
                    <Button
                        type="submit"
                        label="Sign in"
                        icon="pi pi-sign-in"
                        :loading="loading"
                    />
                </form>
                <p class="mt-6 text-center text-xs text-muted-color">Demo: admin@example.com / changethis</p></template
            >
        </Card>
    </section>
</template>
