<script setup lang="ts">
import { ref } from "vue";
import Button from "primevue/button";
import Card from "primevue/card";
import InputText from "primevue/inputtext";
import Password from "primevue/password";
import { api } from "../api";
import type { User } from "../types";
const props = defineProps<{ user: User }>();
const emit = defineEmits<{ updated: [user: User] }>();
const name = ref(props.user.full_name);
const email = ref("");
const currentPassword = ref("");
const busy = ref(false);
const notice = ref("");
const error = ref("");
async function saveName() {
  busy.value = true;
  error.value = "";
  notice.value = "";
  try {
    const user = await api<User>("/users/me", {
      method: "PUT",
      body: JSON.stringify({ full_name: name.value }),
    });
    emit("updated", user);
    notice.value = "Name saved. A confirmation email is on its way.";
  } catch (e) {
    error.value = e instanceof Error ? e.message : "Request failed.";
  } finally {
    busy.value = false;
  }
}
async function changeEmail() {
  busy.value = true;
  error.value = "";
  notice.value = "";
  try {
    await api("/users/me/email-change", {
      method: "POST",
      body: JSON.stringify({
        email: email.value,
        current_password: currentPassword.value,
      }),
    });
    currentPassword.value = "";
    notice.value =
      "Check your new address for a confirmation link. Your current address remains active until then.";
  } catch (e) {
    error.value = e instanceof Error ? e.message : "Request failed.";
  } finally {
    busy.value = false;
  }
}
</script>
<template>
  <div class="grid gap-6 lg:grid-cols-2">
    <Card class="rounded-2xl border border-surface shadow-none">
      <template #title>Profile</template>
      <template #content>
        <form class="grid gap-4" @submit.prevent="saveName">
          <label class="grid gap-2"
            >Display name<InputText v-model="name" required maxlength="200"
          /></label>
          <p class="text-sm text-muted-color">
            Current email: {{ user.email }}
          </p>
          <Button type="submit" label="Save name" :loading="busy" />
        </form>
      </template>
    </Card>
    <Card class="rounded-2xl border border-surface shadow-none">
      <template #title>Change login email</template>
      <template #content>
        <form class="grid gap-4" @submit.prevent="changeEmail">
          <label class="grid gap-2"
            >New email<InputText
              v-model="email"
              type="email"
              required
              autocomplete="email"
          /></label>
          <label class="grid gap-2"
            >Current password<Password
              v-model="currentPassword"
              :feedback="false"
              fluid
              required
              autocomplete="current-password"
          /></label>
          <Button type="submit" label="Send confirmation" :loading="busy" />
        </form>
      </template>
    </Card>
    <p v-if="notice" role="status" class="text-green-600">{{ notice }}</p>
    <p v-if="error" role="alert" class="text-red-600">{{ error }}</p>
  </div>
</template>
