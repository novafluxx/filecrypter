<!-- App.vue - Main Application Component -->
<!--
  This is the root component of the FileCrypter application.

  Features:
  - Tab-based navigation (Encrypt / Decrypt / Batch / Help)
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
import HelpTab from './components/HelpTab.vue';
import { useTheme } from './composables/useTheme';

// Active tab state: 'encrypt', 'decrypt', 'batch', or 'help'
const activeTab = ref<'encrypt' | 'decrypt' | 'batch' | 'help'>('encrypt');

// Theme management
const { theme, toggleTheme } = useTheme();

/**
 * Switch between tabs
 *
 * @param tab - Tab to activate ('encrypt', 'decrypt', 'batch', or 'help')
 */
function switchTab(tab: 'encrypt' | 'decrypt' | 'batch' | 'help') {
  activeTab.value = tab;
}

</script>

<template>
  <div class="app-container">
    <!-- Toolbar -->
    <div class="app-toolbar">
      <h1 class="app-title">FileCrypter</h1>
      <button
        class="theme-toggle"
        @click="toggleTheme"
        :title="theme === 'light' ? 'Switch to dark mode' : 'Switch to light mode'"
      >
        <!-- Sun icon for dark mode (click to go light) -->
        <svg v-if="theme === 'dark'" xmlns="http://www.w3.org/2000/svg" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
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
        <svg v-else xmlns="http://www.w3.org/2000/svg" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <path d="M21 12.79A9 9 0 1 1 11.21 3 7 7 0 0 0 21 12.79z"></path>
        </svg>
      </button>
    </div>

    <!-- Tab Navigation -->
    <div class="tabs">
      <button
        class="tab-button"
        :class="{ active: activeTab === 'encrypt' }"
        @click="switchTab('encrypt')"
        title="Switch to file encryption"
      >
        Encrypt
      </button>
      <button
        class="tab-button"
        :class="{ active: activeTab === 'decrypt' }"
        @click="switchTab('decrypt')"
        title="Switch to file decryption"
      >
        Decrypt
      </button>
      <button
        class="tab-button"
        :class="{ active: activeTab === 'batch' }"
        @click="switchTab('batch')"
        title="Switch to batch processing"
      >
        Batch
      </button>
      <button
        class="tab-button"
        :class="{ active: activeTab === 'help' }"
        @click="switchTab('help')"
        title="Open the FileCrypter user guide"
      >
        Help
      </button>
    </div>

    <!-- Tab Content Area -->
    <div class="tab-panels">
      <!-- Encrypt Tab Panel -->
      <div v-if="activeTab === 'encrypt'" class="tab-panel">
        <EncryptTab />
      </div>

      <!-- Decrypt Tab Panel -->
      <div v-if="activeTab === 'decrypt'" class="tab-panel">
        <DecryptTab />
      </div>

      <!-- Batch Tab Panel -->
      <div v-if="activeTab === 'batch'" class="tab-panel">
        <BatchTab />
      </div>

      <!-- Help Tab Panel -->
      <div v-if="activeTab === 'help'" class="tab-panel">
        <HelpTab />
      </div>
    </div>
  </div>
</template>

<style>
/* Global styles */
/* These apply to the entire application */

/* CSS Variables for theming - Desktop-first simplified system */
:root {
  /* Core variables - Light theme */
  --bg: #f5f5f5;              /* Window background */
  --panel: #ffffff;            /* Primary panels */
  --panel-alt: #fafafa;        /* Hover states, secondary panels */
  --field: #ffffff;            /* Input backgrounds */
  --border: #e0e0e0;          /* All borders */
  --text: #1f1f1f;            /* Primary text */
  --muted: #737373;           /* Secondary text, disabled */
  --accent: #0066cc;          /* Interactive elements */

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
}

[data-theme="dark"] {
  /* Core variables - Dark theme */
  --bg: #1e1e1e;
  --panel: #2a2a2a;
  --panel-alt: #323232;
  --field: #2a2a2a;
  --border: #404040;
  --text: #e0e0e0;
  --muted: #909090;
  --accent: #4a9eff;

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
}

* {
  margin: 0;
  padding: 0;
  box-sizing: border-box;
}

body {
  font-family: system-ui, -apple-system, 'Segoe UI', 'Roboto', 'Ubuntu',
    'Cantarell', 'Noto Sans', sans-serif;
  -webkit-font-smoothing: antialiased;
  -moz-osx-font-smoothing: grayscale;
  background-color: var(--bg);
  color: var(--text);
  font-size: 19px;
  line-height: 1.5;
  transition: background-color 0.3s, color 0.3s;
}

#app {
  height: 100vh;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}
</style>

<style scoped>
/* Desktop-first layout styles */

.app-container {
  display: flex;
  flex-direction: column;
  height: 100vh;
  background: var(--bg);
  overflow: hidden;
}

/* Toolbar Styles */
.app-toolbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  height: 48px;
  padding: 0 16px;
  background: var(--panel);
  border-bottom: 1px solid var(--border);
  flex-shrink: 0;
}

.app-title {
  font-size: 19px;
  font-weight: 600;
  color: var(--text);
  letter-spacing: 0;
  margin: 0;
}

.theme-toggle {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 28px;
  height: 28px;
  border: none;
  border-radius: 4px;
  background: transparent;
  color: var(--muted);
  cursor: pointer;
  transition: all 0.15s;
}

.theme-toggle:hover {
  background: var(--panel-alt);
  color: var(--text);
}

.theme-toggle:active {
  background: var(--border);
}

/* Desktop Tab Navigation */
.tabs {
  display: flex;
  gap: 0;
  background: var(--panel);
  border-bottom: 1px solid var(--border);
  padding: 0 16px;
  flex-shrink: 0;
}

.tab-button {
  padding: 8px 16px;
  background: transparent;
  border: none;
  border-bottom: 2px solid transparent;
  cursor: pointer;
  font-size: 17px;
  font-weight: 500;
  color: var(--muted);
  transition: all 0.15s;
  font-family: inherit;
}

.tab-button:hover:not(.active) {
  color: var(--text);
  background: var(--panel-alt);
}

.tab-button.active {
  color: var(--accent);
  border-bottom-color: var(--accent);
  background: transparent;
}

.tab-button:active {
  background: var(--border);
}

.tab-button:focus-visible {
  outline: 2px solid var(--accent);
  outline-offset: -2px;
}

/* Tab Content Area */
.tab-panels {
  flex: 1;
  overflow-y: auto;
  background: var(--bg);
}

.tab-panel {
  animation: fadeIn 0.15s ease-out;
}

@keyframes fadeIn {
  from {
    opacity: 0;
    transform: translateY(4px);
  }
  to {
    opacity: 1;
    transform: translateY(0);
  }
}
</style>
