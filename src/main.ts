// main.ts - Vue Application Entry Point
//
// This file initializes and mounts the Vue application.
// It's the first JavaScript file that runs when the app starts.
//
// Vue 3 uses createApp() instead of new Vue() from Vue 2
// TypeScript provides type safety throughout the application

import { createApp } from 'vue';
import App from './App.vue';
import './shared.css';

// Create the Vue application instance
// App.vue is the root component
const app = createApp(App);

// Mount the app to the DOM
// The #app element is defined in index.html
app.mount('#app');
