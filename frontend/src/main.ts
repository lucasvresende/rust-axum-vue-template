import { createApp } from 'vue'
import PrimeVue from 'primevue/config'
import ToastService from 'primevue/toastservice'
import Aura from '@primeuix/themes/aura'
import 'primeicons/primeicons.css'
import App from './App.vue'
import './style.css'
createApp(App).use(PrimeVue, { theme: { preset: Aura, options: { darkModeSelector: '.dark' } } }).use(ToastService).mount('#app')
