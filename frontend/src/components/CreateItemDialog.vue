<script setup lang="ts">
import Button from "primevue/button";
import Dialog from "primevue/dialog";
import InputText from "primevue/inputtext";

defineProps<{ editing?: boolean; saving?: boolean }>();
const visible = defineModel<boolean>("visible", { required: true });
const title = defineModel<string>("title", { required: true });
const description = defineModel<string>("description", { required: true });
const quantity = defineModel<number>("quantity", { required: true });
const emit = defineEmits<{ submit: [] }>();
</script>

<template>
  <Dialog
    v-model:visible="visible"
    modal
    :header="editing ? 'Edit item' : 'Create item'"
    :closable="!saving"
    :closeOnEscape="!saving"
    class="w-[calc(100vw-2rem)] max-w-md"
  >
    <form @submit.prevent="emit('submit')">
      <fieldset :disabled="saving" class="grid min-w-0 gap-5">
        <label class="grid gap-2 text-sm font-medium"
          >Title<InputText v-model="title" autofocus required
        /></label>
        <label class="grid gap-2 text-sm font-medium"
          >Description<InputText v-model="description"
        /></label>
        <label class="grid gap-2 text-sm font-medium"
          >Quantity<input
            class="rounded-border border border-surface bg-surface-0 px-3 py-2 text-color focus:outline-2 focus:outline-primary dark:bg-surface-950"
            v-model.number="quantity"
            type="number"
            min="0"
            max="2147483647"
            step="1"
            required
        /></label>
        <p class="text-sm text-muted-color">
          Creation date is recorded automatically.
        </p>
        <Button
          type="submit"
          :label="editing ? 'Save changes' : 'Create item'"
          :loading="saving"
        />
      </fieldset>
    </form>
  </Dialog>
</template>
