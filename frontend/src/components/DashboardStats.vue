<script setup lang="ts">
import Card from "primevue/card";

defineProps<{ itemCount: number; isSuperuser: boolean }>();
</script>

<template>
    <div class="stats">
        <Card>
            <template #content>
                <span>MY ITEMS</span>
                <strong>{{ itemCount }}</strong>
            </template>
        </Card>
        <Card>
            <template #content>
                <span>ACCOUNT</span>
                <strong>{{
                    isSuperuser ? "Admin" : "Member"
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
</template>
