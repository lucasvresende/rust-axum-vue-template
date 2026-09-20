import { defineConfig } from "vite";
import tailwindcss from "@tailwindcss/vite";
import vue from "@vitejs/plugin-vue";
export default defineConfig({
  plugins: [vue(), tailwindcss()],
  server: { proxy: { "/api": "http://localhost:8000" } },
});
