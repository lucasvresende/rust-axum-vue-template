<script setup lang="ts">
import Card from "primevue/card";
import DataTable from "primevue/datatable";
import Column from "primevue/column";
import type { User } from "../types";

defineProps<{ users: User[] }>();
</script>

<template>
    <Card
        class="users"
    >
        <template #title>Users</template>
        <template #content>
            <DataTable :value="users" size="small">
                <Column field="full_name" header="Name" />
                <Column field="email" header="Email" />
                <Column header="Role">
                    <template #body="s">
                        {{ s.data.is_superuser ? "Admin" : "Member" }}
                    </template>
                </Column>
            </DataTable>
        </template>
    </Card>
</template>
