import { createRouter, createWebHistory } from "vue-router";
import App from "./App.vue";
import { session } from "./api";

export const router = createRouter({
  history: createWebHistory(),
  routes: [
    { path: "/login", name: "login", component: App },
    { path: "/forgot-password", name: "forgot", component: App },
    { path: "/reset-password", name: "reset", component: App },
    { path: "/set-password", name: "setup", component: App },
    { path: "/confirm-email", name: "confirmEmail", component: App },
    { path: "/", redirect: "/overview" },
    { path: "/overview", name: "overview", component: App },
    { path: "/items", name: "items", component: App },
    { path: "/users", name: "users", component: App },
    { path: "/profile", name: "profile", component: App },
    { path: "/:pathMatch(.*)*", redirect: "/overview" },
  ],
});

router.beforeEach((to) => {
  const publicPages = ["login", "forgot", "reset", "setup", "confirmEmail"];
  if (!publicPages.includes(String(to.name)) && !session.token) {
    return { name: "login", query: { redirect: to.fullPath }, replace: true };
  }
  if (to.name === "login" && session.token) {
    return { name: "overview", replace: true };
  }
});
