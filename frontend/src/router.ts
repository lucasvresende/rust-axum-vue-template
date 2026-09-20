import { createRouter, createWebHistory } from "vue-router";
import App from "./App.vue";
import { session } from "./api";

export const router = createRouter({
  history: createWebHistory(),
  routes: [
    { path: "/login", name: "login", component: App },
    { path: "/", redirect: "/overview" },
    { path: "/overview", name: "overview", component: App },
    { path: "/items", name: "items", component: App },
    { path: "/users", name: "users", component: App },
    { path: "/:pathMatch(.*)*", redirect: "/overview" },
  ],
});

router.beforeEach((to) => {
  if (to.name !== "login" && !session.token) {
    return { name: "login", query: { redirect: to.fullPath }, replace: true };
  }
  if (to.name === "login" && session.token) {
    return { name: "overview", replace: true };
  }
});
