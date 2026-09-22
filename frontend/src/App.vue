<script setup lang="ts">
import { computed, onMounted, ref, watch } from "vue";
import { useRoute, useRouter } from "vue-router";
import Button from "primevue/button";
import Dialog from "primevue/dialog";
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
  quantity = ref(1),
  dialog = ref(false),
  loading = ref(false),
  dark = ref(false);

const editingItem = ref<Item | null>(null);
const saving = ref(false);
const deleting = ref(false);
const deleteDialog = ref(false);
const pendingDelete = ref<Item | null>(null);

/** Initialize the shared editor with an existing item or new-item defaults. */
function openItem(item: Item | null = null) {
  editingItem.value = item;
  title.value = item?.title ?? "";
  description.value = item?.description ?? "";
  quantity.value = item?.quantity ?? 1;
  dialog.value = true;
}

function requestDelete(item: Item) {
  pendingDelete.value = item;
  deleteDialog.value = true;
}

const sidebarVisible = ref(true);
watch(
  dark,
  (value) => document.documentElement.classList.toggle("app-dark", value),
  {
    immediate: true,
  },
);

const route = useRoute();
const router = useRouter();
const activeView = computed<WorkspaceView>(() =>
  route.name === "items" || route.name === "users" ? route.name : "overview",
);

/** Allow only known workspace paths as post-login destinations. */
function loginDestination() {
  const redirect = route.query.redirect;
  return typeof redirect === "string" &&
    ["/overview", "/items", "/users"].includes(redirect)
    ? redirect
    : "/overview";
}

watch([activeView, user], ([view, currentUser]) => {
  if (view === "users" && currentUser && !currentUser.is_superuser) {
    void router.replace({ name: "overview" });
  }
});
watch(activeView, () => {
  dialog.value = false;
  deleteDialog.value = false;
  pendingDelete.value = null;
});

function notify(severity: "success" | "error", detail: string) {
  toast.add({
    severity,
    summary: severity === "success" ? "Done" : "Something went wrong",
    detail,
    life: 3500,
  });
}

/** Refresh the profile and owned items, loading the user directory only for admins. */
async function load() {
  user.value = await api<User>("/users/me");
  items.value = await api<Item[]>("/items");
  if (user.value.is_superuser) users.value = await api<User[]>("/users");
}

/** Store the token, load the workspace, and resume an allowed destination. */
async function login() {
  if (loading.value) return;
  const destination = loginDestination();
  loading.value = true;
  try {
    const t = await api<{ access_token: string }>("/login", {
      method: "POST",
      body: JSON.stringify({ email: email.value, password: password.value }),
    });
    session.token = t.access_token;
    await load();
    await router.replace(
      destination === "/users" && !user.value?.is_superuser
        ? "/overview"
        : destination,
    );
    notify("success", "Welcome back.");
  } catch (e: any) {
    clearSession();
    notify("error", e.message);
  } finally {
    loading.value = false;
  }
}

/** Create or update the selected item, suppressing duplicate submissions. */
async function saveItem() {
  if (saving.value) return;
  saving.value = true;
  const id = editingItem.value?.id;
  try {
    await api(id ? `/items/${id}` : "/items", {
      method: id ? "PUT" : "POST",
      body: JSON.stringify({
        title: title.value,
        description: description.value,
        quantity: quantity.value,
      }),
    });
    title.value = "";
    description.value = "";
    quantity.value = 1;
    dialog.value = false;
    await load();
    notify("success", id ? "Item updated." : "Item created.");
  } catch (e: any) {
    notify("error", e.message);
  } finally {
    saving.value = false;
  }
}

/** Delete the confirmed item and refresh the workspace after success. */
async function remove() {
  if (!pendingDelete.value || deleting.value) return;
  deleting.value = true;
  const id = pendingDelete.value.id;
  try {
    await api("/items/" + id, { method: "DELETE" });
    deleteDialog.value = false;
    pendingDelete.value = null;
    await load();
    notify("success", "Item deleted.");
  } catch (e: any) {
    notify("error", e.message);
  } finally {
    deleting.value = false;
  }
}

/** Clear persisted authentication, user data, and pending dialogs together. */
function clearSession() {
  dialog.value = false;
  deleteDialog.value = false;
  pendingDelete.value = null;
  session.token = null;
  user.value = null;
  items.value = [];
  users.value = [];
}

function logout() {
  clearSession();
  void router.replace({ name: "login" });
}

onMounted(async () => {
  if (session.token)
    try {
      await load();
    } catch {
      const destination = route.fullPath;
      clearSession();
      await router.replace({ name: "login", query: { redirect: destination } });
    }
});
</script>

<template>
  <Toast class="max-w-[calc(100vw-2.5rem)]" />
  <main class="min-h-screen">
    <LoginForm
      v-if="route.name === 'login'"
      v-model:email="email"
      v-model:password="password"
      :loading="loading"
      @submit="login"
    />
    <template v-else-if="user">
      <AppHeader
        v-model:dark="dark"
        :sidebar-visible="sidebarVisible"
        :user="user"
        @home="router.push({ name: 'overview' })"
        @toggle-menu="sidebarVisible = !sidebarVisible"
        @logout="logout"
      />
      <div
        class="mx-auto flex max-w-[1600px] flex-col gap-6 p-4 lg:flex-row lg:p-8"
      >
        <WorkspaceNav
          v-if="sidebarVisible"
          :active-view="activeView"
          :is-superuser="user.is_superuser"
        />
        <section class="min-w-0 flex-1 space-y-6">
          <DashboardHero
            :user="user"
            :active-view="activeView"
            @create="openItem()"
          />
          <DashboardStats
            v-if="activeView === 'overview'"
            :item-count="items.length"
            :is-superuser="user.is_superuser"
          />
          <ItemsTable
            v-if="activeView !== 'users'"
            :items="items"
            @remove="requestDelete"
            @edit="openItem"
          />
          <UsersTable
            v-if="user.is_superuser && activeView !== 'items'"
            :users="users"
          />
          <footer class="py-4 text-center text-sm text-muted-color">
            Axum + Vue <span class="mx-2">·</span> Admin workspace
          </footer>
        </section>
      </div>
    </template>
  </main>
  <Dialog
    v-model:visible="deleteDialog"
    modal
    header="Delete item"
    :closable="!deleting"
    :closeOnEscape="!deleting"
    class="w-[calc(100vw-2rem)] max-w-md"
  >
    <p>
      Delete <strong>{{ pendingDelete?.title }}</strong
      >? This action cannot be undone.
    </p>
    <template #footer>
      <Button
        label="Cancel"
        severity="secondary"
        outlined
        autofocus
        :disabled="deleting"
        @click="deleteDialog = false"
      />
      <Button
        label="Delete item"
        severity="danger"
        :loading="deleting"
        @click="remove"
      />
    </template>
  </Dialog>
  <CreateItemDialog
    v-model:visible="dialog"
    v-model:title="title"
    v-model:description="description"
    v-model:quantity="quantity"
    :editing="!!editingItem"
    :saving="saving"
    @submit="saveItem"
  />
</template>
