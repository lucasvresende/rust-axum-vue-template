import { createApp, h } from "vue";
import PrimeVue from "primevue/config";
import ToastService from "primevue/toastservice";
import Aura from "@primeuix/themes/aura";
import "primeicons/primeicons.css";
import { RouterView } from "vue-router";
import { router } from "./router";
import "./style.css";

createApp({ render: () => h(RouterView) })
  .use(router)
  .use(PrimeVue, {
    theme: {
      preset: Aura,
      options: {
        darkModeSelector: ".app-dark",
        cssLayer: {
          name: "primevue",
          order: "theme, base, primevue, components, utilities",
        },
      },
    },
  })
  .use(ToastService)
  .mount("#app");
