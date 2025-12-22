<!-- App.vue - Main Application Component -->
<!--
  This is the root component of the FileCypter application.

  Features:
  - Tab-based navigation (Encrypt / Decrypt)
  - Responsive layout
  - Clean, modern design
  - Header with branding

  Vue Composition API:
  - ref() for reactive tab state
  - Component composition with EncryptTab and DecryptTab
-->

<script setup lang="ts">
import { ref } from 'vue';
import EncryptTab from './components/EncryptTab.vue';
import DecryptTab from './components/DecryptTab.vue';
import BatchTab from './components/BatchTab.vue';
import { useTheme } from './composables/useTheme';

// Active tab state: 'encrypt', 'decrypt', or 'batch'
const activeTab = ref<'encrypt' | 'decrypt' | 'batch'>('encrypt');

// Theme management
const { theme, toggleTheme } = useTheme();

/**
 * Switch between tabs
 *
 * @param tab - Tab to activate ('encrypt', 'decrypt', or 'batch')
 */
function switchTab(tab: 'encrypt' | 'decrypt' | 'batch') {
  activeTab.value = tab;
}
</script>

<template>
  <div class="app-container">
    <!-- Header Section -->
    <header class="app-header">
      <div class="header-row">
        <h1 class="app-title">FileCypter</h1>
        <button
          class="theme-toggle"
          @click="toggleTheme"
          :title="theme === 'light' ? 'Switch to dark mode' : 'Switch to light mode'"
        >
          <!-- Sun icon for dark mode (click to go light) -->
          <svg v-if="theme === 'dark'" xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <circle cx="12" cy="12" r="5"></circle>
            <line x1="12" y1="1" x2="12" y2="3"></line>
            <line x1="12" y1="21" x2="12" y2="23"></line>
            <line x1="4.22" y1="4.22" x2="5.64" y2="5.64"></line>
            <line x1="18.36" y1="18.36" x2="19.78" y2="19.78"></line>
            <line x1="1" y1="12" x2="3" y2="12"></line>
            <line x1="21" y1="12" x2="23" y2="12"></line>
            <line x1="4.22" y1="19.78" x2="5.64" y2="18.36"></line>
            <line x1="18.36" y1="5.64" x2="19.78" y2="4.22"></line>
          </svg>
          <!-- Moon icon for light mode (click to go dark) -->
          <svg v-else xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="M21 12.79A9 9 0 1 1 11.21 3 7 7 0 0 0 21 12.79z"></path>
          </svg>
        </button>
      </div>
      <p class="app-subtitle">Secure File Encryption</p>
    </header>

    <!-- Tab Navigation -->
    <div class="tabs">
      <button
        class="tab-button"
        :class="{ active: activeTab === 'encrypt' }"
        @click="switchTab('encrypt')"
      >
        Encrypt
      </button>
      <button
        class="tab-button"
        :class="{ active: activeTab === 'decrypt' }"
        @click="switchTab('decrypt')"
      >
        Decrypt
      </button>
      <button
        class="tab-button"
        :class="{ active: activeTab === 'batch' }"
        @click="switchTab('batch')"
      >
        Batch
      </button>
    </div>

    <!-- Tab Content Area -->
    <div class="tab-panels">
      <!-- Encrypt Tab Panel -->
      <div v-show="activeTab === 'encrypt'" class="tab-panel">
        <EncryptTab />
      </div>

      <!-- Decrypt Tab Panel -->
      <div v-show="activeTab === 'decrypt'" class="tab-panel">
        <DecryptTab />
      </div>

      <!-- Batch Tab Panel -->
      <div v-show="activeTab === 'batch'" class="tab-panel">
        <BatchTab />
      </div>
    </div>

    <!-- Footer (optional) -->
    <footer class="app-footer">
      <p class="footer-text">
        Built with Tauri, Rust, and Vue 3 |
        AES-256-GCM + Argon2id
      </p>
    </footer>
  </div>
</template>

<style>
/* Global styles */
/* These apply to the entire application */

/* CSS Variables for theming */
:root {
  /* Light theme (default) */
  --bg-primary: #f5f7fa;
  --bg-secondary: white;
  --bg-tertiary: #f9f9f9;
  --text-primary: #333;
  --text-secondary: #666;
  --text-muted: #999;
  --border-color: #e0e0e0;
  --accent-primary: #4a90e2;
  --accent-secondary: #7b68ee;
  --accent-hover: #3a7bc8;

  /* Status colors */
  --success-bg: #d4edda;
  --success-text: #155724;
  --success-border: #c3e6cb;
  --error-bg: #f8d7da;
  --error-text: #721c24;
  --error-border: #f5c6cb;
  --info-bg: #d1ecf1;
  --info-text: #0c5460;
  --info-border: #bee5eb;
  --warning-bg: #fff3cd;
  --warning-text: #856404;
  --warning-border: #ffeaa7;

  /* Component colors */
  --input-bg: #f9f9f9;
  --btn-secondary-bg: #6c757d;
  --btn-secondary-hover: #5a6268;
}

[data-theme="dark"] {
  /* Dark theme overrides */
  --bg-primary: #1a1a2e;
  --bg-secondary: #16213e;
  --bg-tertiary: #1f1f3a;
  --text-primary: #e0e0e0;
  --text-secondary: #a0a0a0;
  --text-muted: #707070;
  --border-color: #2a2a4a;
  --accent-primary: #5a9cf2;
  --accent-secondary: #8b78ff;
  --accent-hover: #4a8ce2;

  /* Status colors - dark variants */
  --success-bg: #1e4a2e;
  --success-text: #7ddf9a;
  --success-border: #2a5a3a;
  --error-bg: #4a1e2e;
  --error-text: #f5a0a0;
  --error-border: #5a2a3a;
  --info-bg: #1e3a4a;
  --info-text: #a0d8f0;
  --info-border: #2a4a5a;
  --warning-bg: #4a3a1e;
  --warning-text: #f0d080;
  --warning-border: #5a4a2a;

  /* Component colors */
  --input-bg: #1f1f3a;
  --btn-secondary-bg: #4a4a6a;
  --btn-secondary-hover: #5a5a7a;
}

* {
  margin: 0;
  padding: 0;
  box-sizing: border-box;
}

body {
  font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', 'Roboto', 'Oxygen',
    'Ubuntu', 'Cantarell', 'Fira Sans', 'Droid Sans', 'Helvetica Neue',
    sans-serif;
  -webkit-font-smoothing: antialiased;
  -moz-osx-font-smoothing: grayscale;
  background-color: var(--bg-primary);
  color: var(--text-primary);
  line-height: 1.6;
  transition: background-color 0.3s, color 0.3s;
}

#app {
  height: 100vh;
  display: flex;
  justify-content: center;
  align-items: center;
  padding: 20px;
}
</style>

<style scoped>
/* Component-specific styles */

.app-container {
  width: 100%;
  max-width: 600px;
  background: var(--bg-secondary);
  border-radius: 12px;
  box-shadow: 0 4px 16px rgba(0, 0, 0, 0.1);
  padding: 32px;
  transition: background-color 0.3s;
}

/* Header Styles */
.app-header {
  text-align: center;
  margin-bottom: 32px;
}

.header-row {
  display: flex;
  justify-content: center;
  align-items: center;
  gap: 12px;
  margin-bottom: 8px;
}

.app-title {
  font-size: 32px;
  font-weight: 700;
  color: var(--accent-primary);
  letter-spacing: -0.5px;
}

.theme-toggle {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 36px;
  height: 36px;
  border: none;
  border-radius: 8px;
  background: var(--bg-tertiary);
  color: var(--text-secondary);
  cursor: pointer;
  transition: all 0.2s;
}

.theme-toggle:hover {
  background: var(--border-color);
  color: var(--accent-primary);
}

.app-subtitle {
  font-size: 16px;
  color: var(--text-secondary);
  font-weight: 400;
}

/* Tab Navigation Styles */
.tabs {
  display: flex;
  gap: 8px;
  margin-bottom: 24px;
  border-bottom: 2px solid var(--border-color);
}

.tab-button {
  flex: 1;
  padding: 12px 24px;
  background: none;
  border: none;
  border-bottom: 3px solid transparent;
  cursor: pointer;
  font-size: 16px;
  font-weight: 500;
  color: var(--text-secondary);
  transition: all 0.2s ease;
  font-family: inherit;
  position: relative;
  bottom: -2px;
}

.tab-button:hover {
  color: var(--accent-primary);
  background-color: var(--bg-tertiary);
}

.tab-button.active {
  color: var(--accent-primary);
  border-bottom-color: var(--accent-primary);
}

/* Tab Content Styles */
.tab-panels {
  min-height: 400px;
}

.tab-panel {
  animation: fadeIn 0.2s ease-in;
}

@keyframes fadeIn {
  from {
    opacity: 0;
    transform: translateY(8px);
  }
  to {
    opacity: 1;
    transform: translateY(0);
  }
}

/* Footer Styles */
.app-footer {
  margin-top: 32px;
  padding-top: 20px;
  border-top: 1px solid var(--border-color);
  text-align: center;
}

.footer-text {
  font-size: 12px;
  color: var(--text-muted);
}

/* Responsive Design */
@media (max-width: 640px) {
  .app-container {
    padding: 24px;
  }

  .app-title {
    font-size: 28px;
  }

  .tab-button {
    font-size: 14px;
    padding: 10px 16px;
  }
}
</style>
