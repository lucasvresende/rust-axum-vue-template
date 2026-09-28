<script setup lang="ts">
import { computed, ref } from "vue";
import { useRoute, RouterLink } from "vue-router";
import Button from "primevue/button";
import Card from "primevue/card";
import InputText from "primevue/inputtext";
import Password from "primevue/password";
import { api, session } from "../api";

const route = useRoute();
const kind = computed(() => String(route.name));
const token = computed(() =>
  typeof route.query.token === "string" ? route.query.token : "",
);
const email = ref("");
const password = ref("");
const confirm = ref("");
const busy = ref(false);
const message = ref("");
const error = ref("");
const heading = computed(
  () =>
    ({
      forgot: "Recover your password",
      reset: "Set a new password",
      setup: "Set your password",
      confirmEmail: "Confirm your email",
    })[kind.value] || "Account",
);

async function submit() {
  if (busy.value) return;
  error.value = "";
  message.value = "";
  if (
    (kind.value === "setup" || kind.value === "reset") &&
    password.value !== confirm.value
  ) {
    error.value = "Passwords do not match.";
    return;
  }
  busy.value = true;
  try {
    if (kind.value === "forgot") {
      const result = await api<{ message: string }>(
        "/password/recovery-requests",
        { method: "POST", body: JSON.stringify({ email: email.value }) },
      );
      message.value = result.message;
    } else if (kind.value === "confirmEmail") {
      if (!token.value) throw new Error("This link is incomplete.");
      await api("/users/confirm-email", {
        method: "POST",
        body: JSON.stringify({ token: token.value }),
      });
      session.token = null;
      message.value = "Email confirmed. Sign in with your new address.";
    } else {
      if (!token.value) throw new Error("This link is incomplete.");
      await api(
        kind.value === "setup" ? "/password/setup" : "/password/recover",
        {
          method: "POST",
          body: JSON.stringify({
            token: token.value,
            password: password.value,
          }),
        },
      );
      session.token = null;
      message.value = "Password saved. You can now sign in.";
      password.value = "";
      confirm.value = "";
    }
  } catch (e) {
    error.value = e instanceof Error ? e.message : "Request failed.";
  } finally {
    busy.value = false;
  }
}
</script>

<template>
  <section
    class="grid min-h-screen place-items-center bg-surface-50 p-4 dark:bg-surface-950"
  >
    <Card
      class="w-full max-w-md rounded-3xl border border-surface p-4 shadow-none sm:p-6"
    >
      <template #title>{{ heading }}</template>
      <template #content>
        <form class="mt-4 grid gap-4" @submit.prevent="submit">
          <label v-if="kind === 'forgot'" class="grid gap-2"
            >Email<InputText
              v-model="email"
              type="email"
              autocomplete="email"
              required
          /></label>
          <template v-if="kind === 'setup' || kind === 'reset'">
            <label class="grid gap-2"
              >New password<Password
                v-model="password"
                :feedback="false"
                fluid
                required
                autocomplete="new-password"
            /></label>
            <label class="grid gap-2"
              >Confirm password<Password
                v-model="confirm"
                :feedback="false"
                fluid
                required
                autocomplete="new-password"
            /></label>
            <p class="text-sm text-muted-color">Use at least 15 characters.</p>
          </template>
          <p v-if="message" role="status" class="text-sm text-green-600">
            {{ message }}
          </p>
          <p v-if="error" role="alert" class="text-sm text-red-600">
            {{ error }}
          </p>
          <Button
            v-if="!message"
            type="submit"
            :label="
              kind === 'forgot'
                ? 'Send recovery link'
                : kind === 'confirmEmail'
                  ? 'Confirm email'
                  : 'Save password'
            "
            :loading="busy"
          />
          <RouterLink to="/login" class="text-center text-sm text-primary"
            >Back to sign in</RouterLink
          >
        </form>
      </template>
    </Card>
  </section>
</template>
