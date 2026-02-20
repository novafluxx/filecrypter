import { createApp } from 'vue';
import PrimeVue from 'primevue/config';
import ConfirmationService from 'primevue/confirmationservice';
import { definePreset } from '@primeuix/themes';
import Aura from '@primeuix/themes/aura';
import App from './App.vue';
import './shared.css';

const AppPreset = definePreset(Aura, {
  semantic: {
    primary: {
      50: '#eff6ff',
      100: '#dbeafe',
      200: '#bfdbfe',
      300: '#93c5fd',
      400: '#60a5fa',
      500: '#3b82f6',
      600: '#2563eb',
      700: '#1d4ed8',
      800: '#1e40af',
      900: '#1e3a8a',
      950: '#172554',
    },
  },
});

const app = createApp(App);

app.use(PrimeVue, {
  theme: {
    preset: AppPreset,
    options: {
      darkModeSelector: '[data-theme="dark"]',
      cssLayer: false,
    },
  },
});

app.use(ConfirmationService);

app.mount('#app');
