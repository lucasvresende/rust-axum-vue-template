<script setup lang="ts">
import Card from "primevue/card";
import DataTable from "primevue/datatable";
import Column from "primevue/column";
import type { User } from "../types";

defineProps<{ users: User[] }>();
</script>

<template>
    <Card
        class="overflow-hidden rounded-2xl border border-surface shadow-none"
    >
        <template #title>Users</template>
        <template #content>
            <DataTable :value="users" paginator :rows="10" :rowsPerPageOptions="[5, 10, 20]" dataKey="id" scrollable>
                <template #empty>No workspace members found.</template>
                <Column field="full_name" header="Name" sortable />
                <Column field="email" header="Email" sortable />
                <Column header="Role">
                    <template #body="s">
                        {{ s.data.is_superuser ? "Admin" : "Member" }}
                    </template>
                </Column>
            </DataTable>
        </template>
    </Card>
</template>
