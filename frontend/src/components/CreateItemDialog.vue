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
        :style="{ width: '28rem' }"
    >
        <form class="stack" @submit.prevent="emit('submit')">
            <label>Title<InputText v-model="title" autofocus required /></label>
            <label>Description<InputText v-model="description" /></label>
            <Button type="submit" label="Create item" />
        </form>
    </Dialog>
</template>
