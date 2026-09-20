<script setup lang="ts">
import Button from "primevue/button";
import Dialog from "primevue/dialog";
import InputText from "primevue/inputtext";

const visible = defineModel<boolean>("visible", { required: true });
const title = defineModel<string>("title", { required: true });
const description = defineModel<string>("description", { required: true });
const emit = defineEmits<{ submit: [] }>();
</script>

<template>
    <Dialog
        v-model:visible="visible"
        modal
        header="Create item"
        class="w-[calc(100vw-2rem)] max-w-md"
    >
        <form class="grid gap-5" @submit.prevent="emit('submit')">
            <label class="grid gap-2 text-sm font-medium">Title<InputText v-model="title" autofocus required /></label>
            <label class="grid gap-2 text-sm font-medium">Description<InputText v-model="description" /></label>
            <Button type="submit" label="Create item" />
        </form>
    </Dialog>
</template>
