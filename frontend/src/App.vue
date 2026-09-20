<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import Button from "primevue/button";
import Card from "primevue/card";
import InputText from "primevue/inputtext";
import Password from "primevue/password";
import Dialog from "primevue/dialog";
import DataTable from "primevue/datatable";
import Column from "primevue/column";
import Toast from "primevue/toast";
import { useToast } from "primevue/usetoast";
import { api, session } from "./api";

const toast = useToast();

const email = ref("admin@example.com"),
    password = ref("changethis"),
    user = ref<any>(null),
    items = ref<any[]>([]),
    users = ref<any[]>([]),
    title = ref(""),
    description = ref(""),
    dialog = ref(false),
    loading = ref(false),
    dark = ref(false);

const activeView = ref<"overview" | "items" | "users">("overview");

const initials = computed(
    () => user.value?.full_name?.slice(0, 1).toUpperCase() || "U",
);

function notify(severity: "success" | "error", detail: string) {
    toast.add({
        severity,
        summary: severity === "success" ? "Done" : "Something went wrong",
        detail,
        life: 3500,
    });
}
async function load() {
    user.value = await api("/users/me");
    items.value = await api("/items");
    if (user.value.is_superuser) users.value = await api("/users");
}
async function login() {
    loading.value = true;
    try {
        const t: any = await api("/login", {
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
        <section v-if="!user" class="login-shell">
            <Card class="login-card">
                <template #title><i class="pi pi-bolt"></i> Axum + Vue</template>
                <template #subtitle>A production-ready starter</template>
                <template #content>
                    <form @submit.prevent="login" class="stack">
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
        <template v-else>
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
                        @click="logout"
                    />
                </div>
            </header>
            <div class="layout">
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
                            v-if="user.is_superuser"
                            type="button"
                            :class="{ active: activeView === 'users' }"
                            :aria-current="activeView === 'users' ? 'page' : undefined"
                            @click="activeView = 'users'"
                        >
                            <i class="pi pi-users"></i> Users
                        </button>
                    </nav>
                </aside>
                <section class="content">
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
                            @click="dialog = true"
                        />
                    </div>
                    <div v-if="activeView === 'overview'" class="stats">
                        <Card>
                            <template #content>
                                <span>MY ITEMS</span>
                                <strong>{{ items.length }}</strong>
                            </template>
                        </Card>
                        <Card>
                            <template #content>
                                <span>ACCOUNT</span>
                                <strong>{{
                                    user.is_superuser ? "Admin" : "Member"
                                }}</strong>
                            </template>
                        </Card>
                        <Card>
                            <template #content>
                                <span>API STATUS</span
                                ><strong class="success">Healthy</strong>
                            </template>
                        </Card>
                    </div>
                    <Card v-if="activeView !== 'users'"
                        ><template #title>My items</template
                        ><template #content
                            ><DataTable
                                :value="items"
                                stripedRows
                                responsiveLayout="scroll"
                                dataKey="id"
                                ><Column field="title" header="Title" /><Column
                                    field="description"
                                    header="Description"
                                /><Column header=""
                                    ><template #body="slotProps"
                                        ><Button
                                            icon="pi pi-trash"
                                            severity="danger"
                                            text
                                            rounded
                                            aria-label="Delete item"
                                            @click="
                                                remove(slotProps.data.id)
                                            " /></template></Column
                                ><template #empty
                                    >No items yet. Create your first one.</template
                                ></DataTable
                            ></template
                        ></Card
                    >
                    <Card
                        v-if="user.is_superuser && activeView !== 'items'"
                        class="users"
                        ><template #title>Users</template
                        ><template #content
                            ><DataTable :value="users" size="small"
                                ><Column field="full_name" header="Name" /><Column
                                    field="email"
                                    header="Email"
                                /><Column header="Role"
                                    ><template #body="s">{{
                                        s.data.is_superuser ? "Admin" : "Member"
                                    }}</template></Column
                                ></DataTable
                            ></template
                        ></Card
                    >
                </section>
            </div>
        </template>
    </main>
    <Dialog
        v-model:visible="dialog"
        modal
        header="Create item"
        :style="{ width: '28rem' }"
        ><form class="stack" @submit.prevent="addItem">
            <label>Title<InputText v-model="title" autofocus required /></label
            ><label>Description<InputText v-model="description" /></label
            ><Button type="submit" label="Create item" /></form
    ></Dialog>
</template>
