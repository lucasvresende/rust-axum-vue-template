<script setup lang="ts">
import Button from "primevue/button";
import Card from "primevue/card";
import DataTable from "primevue/datatable";
import Column from "primevue/column";
import type { Item } from "../types";

defineProps<{ items: Item[] }>();
const emit = defineEmits<{ remove: [id: string] }>();
</script>

<template>
    <Card class="overflow-hidden rounded-2xl border border-surface shadow-none">
        <template #title>My items</template>
        <template #content>
            <DataTable
                :value="items"
                paginator :rows="10" :rowsPerPageOptions="[5, 10, 20]"
                responsiveLayout="scroll"
                dataKey="id"
            >
                <Column field="title" header="Title" sortable />
                <Column field="description" header="Description" />
                <Column header="">
                    <template #body="slotProps">
                        <Button
                            icon="pi pi-trash"
                            severity="danger"
                            text
                            rounded
                            aria-label="Delete item"
                            @click="emit('remove', slotProps.data.id)"
                        />
                    </template>
                </Column>
                <template #empty>
                    No items yet. Create your first one.
                </template>
            </DataTable>
        </template>
    </Card>
</template>
