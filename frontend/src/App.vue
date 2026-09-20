<script setup lang="ts">
import { onMounted, ref } from "vue";
import Toast from "primevue/toast";
import { useToast } from "primevue/usetoast";
import { api, session } from "./api";

import LoginForm from "./components/LoginForm.vue";
import AppHeader from "./components/AppHeader.vue";
import WorkspaceNav from "./components/WorkspaceNav.vue";
import DashboardHero from "./components/DashboardHero.vue";
import DashboardStats from "./components/DashboardStats.vue";
import ItemsTable from "./components/ItemsTable.vue";
import UsersTable from "./components/UsersTable.vue";
import CreateItemDialog from "./components/CreateItemDialog.vue";
import type { Item, User, WorkspaceView } from "./types";

const toast = useToast();

const email = ref("admin@example.com"),
    password = ref("changethis"),
    user = ref<User | null>(null),
    items = ref<Item[]>([]),
    users = ref<User[]>([]),
    title = ref(""),
    description = ref(""),
    dialog = ref(false),
    loading = ref(false),
    dark = ref(false);

const activeView = ref<WorkspaceView>("overview");

function notify(severity: "success" | "error", detail: string) {
    toast.add({
        severity,
        summary: severity === "success" ? "Done" : "Something went wrong",
        detail,
        life: 3500,
    });
}
async function load() {
    user.value = await api<User>("/users/me");
    items.value = await api<Item[]>("/items");
    if (user.value.is_superuser) users.value = await api<User[]>("/users");
}
async function login() {
    loading.value = true;
    try {
        const t = await api<{ access_token: string }>("/login", {
            method: "POST",
            body: JSON.stringify({ email: email.value, password: password.value }),
        });
        session.token = t.access_token;
        await load();
        notify("success", "Welcome back.");
    } catch (e: any) {
        notify("error", e.message);
    } finally {
        loading.value = false;
    }
}
async function addItem() {
    try {
        await api("/items", {
            method: "POST",
            body: JSON.stringify({
                title: title.value,
                description: description.value,
            }),
        });
        title.value = "";
        description.value = "";
        dialog.value = false;
        await load();
        notify("success", "Item created.");
    } catch (e: any) {
        notify("error", e.message);
    }
}
async function remove(id: string) {
    try {
        await api("/items/" + id, { method: "DELETE" });
        await load();
        notify("success", "Item deleted.");
    } catch (e: any) {
        notify("error", e.message);
    }
}
function logout() {
    activeView.value = "overview";
    session.token = null;
    user.value = null;
    items.value = [];
    users.value = [];
}
onMounted(async () => {
    if (session.token)
        try {
            await load();
        } catch {
            logout();
        }
});
</script>

<template>
    <Toast />
    <main :class="{ dark }">
        <LoginForm
            v-if="!user"
            v-model:email="email"
            v-model:password="password"
            :loading="loading"
            @submit="login"
        />
        <template v-else>
            <AppHeader v-model:dark="dark" :user="user" @logout="logout" />
            <div class="layout">
                <WorkspaceNav v-model="activeView" :is-superuser="user.is_superuser" />
                <section class="content">
                    <DashboardHero :user="user" :active-view="activeView" @create="dialog = true" />
                    <DashboardStats
                        v-if="activeView === 'overview'"
                        :item-count="items.length"
                        :is-superuser="user.is_superuser"
                    />
                    <ItemsTable v-if="activeView !== 'users'" :items="items" @remove="remove" />
                    <UsersTable v-if="user.is_superuser && activeView !== 'items'" :users="users" />
                </section>
            </div>
        </template>
    </main>
    <CreateItemDialog
        v-model:visible="dialog"
        v-model:title="title"
        v-model:description="description"
        @submit="addItem"
    />
</template>
