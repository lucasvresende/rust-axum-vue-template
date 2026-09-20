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
    <section class="login-shell">
        <Card class="login-card">
            <template #title><i class="pi pi-bolt"></i> Axum + Vue</template>
            <template #subtitle>A production-ready starter</template>
            <template #content>
                <form @submit.prevent="emit('submit')" class="stack">
                    <label>
                        Email<InputText
                            v-model="email"
                            type="email"
                            autocomplete="email"
                        />
                    </label>
                    <label>
                        Password<Password
                            v-model="password"
                            :feedback="false"
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
                <p class="hint">Demo: admin@example.com / changethis</p></template
            >
        </Card>
    </section>
</template>
