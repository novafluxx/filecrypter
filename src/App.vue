<!-- App.vue - Main Application Component -->
<!--
  This is the root component of the FileCrypter application.

  Features:
  - Tab-based navigation (Encrypt / Decrypt / Batch / Help)
  - Responsive layout
  - Clean, modern design

  Vue Composition API:
  - ref() for reactive tab state
  - Component composition with EncryptTab and DecryptTab
-->

<script setup lang="ts">
import { ref, onMounted, onUnmounted, watch } from 'vue';
import Tabs from 'primevue/tabs';
import TabList from 'primevue/tablist';
import Tab from 'primevue/tab';
import ConfirmDialog from 'primevue/confirmdialog';
import EncryptTab from './components/EncryptTab.vue';
import DecryptTab from './components/DecryptTab.vue';
import BatchTab from './components/BatchTab.vue';
import SettingsTab from './components/SettingsTab.vue';
import HelpTab from './components/HelpTab.vue';
import BottomNav from './components/BottomNav.vue';
import UpdateNotification from './components/UpdateNotification.vue';
import ChangelogAction from './components/ChangelogAction.vue';
import { useTheme } from './composables/useTheme';
import { useSettings } from './composables/useSettings';
import { usePlatform } from './composables/usePlatform';
import { useUpdater } from './composables/useUpdater';
import { useVersion } from './composables/useVersion';
import type { TabName } from './types/tabs';

// Active tab state
const activeTab = ref<TabName>('encrypt');

// Initialize theme (applies theme from settings)
// appliedTheme is 'light' or 'dark' (resolved from system preference if needed)
useTheme();

// Platform detection for conditional navigation
// isInitialized prevents UI flash before detection completes
const { isMobile, isInitialized } = usePlatform();

// Settings management
const { initSettings } = useSettings();

// Auto-updater (desktop only)
const { checkForUpdates } = useUpdater();

// App version (desktop only)
const { version } = useVersion();

// Context menu handler (stored for cleanup)
const preventContextMenu = (event: Event) => event.preventDefault();

// Initialize settings store on mount
onMounted(async () => {
  await initSettings();

  // Disable context menu (right-click) for desktop-like feel
  document.addEventListener('contextmenu', preventContextMenu);

  // Check for updates on desktop platforms (not on mobile - app stores handle updates)
  // Use a watcher to ensure platform detection completes before checking
  watch(isInitialized, (initialized) => {
    if (initialized && !isMobile.value) {
      // Delay slightly to not block initial render
      window.setTimeout(async () => {
        try {
          await checkForUpdates();
        } catch (err) {
          // Silently fail - update check is non-critical
          console.warn('Update check failed:', err);
        }
      }, 2000);
    }
  }, { immediate: true });
});

// Clean up global event listeners
onUnmounted(() => {
  document.removeEventListener('contextmenu', preventContextMenu);
});

/**
 * Switch between tabs
 *
 * @param tab - Tab to activate
 */
function switchTab(tab: string | number) {
  activeTab.value = tab as TabName;
}

</script>

<template>
  <!-- Global ConfirmDialog (replaces NDialogProvider) -->
  <ConfirmDialog />

  <!-- Update notification banner (desktop only) -->
  <UpdateNotification v-if="isInitialized && !isMobile" />

  <div class="app-container">
    <!-- Desktop Header with Tab Navigation (hidden on mobile, waits for platform detection) -->
    <div v-if="isInitialized && !isMobile" class="desktop-header">
      <Tabs
        :value="activeTab"
        :scrollable="true"
        @update:value="switchTab"
        class="desktop-tabs"
      >
        <TabList>
          <Tab value="encrypt">Encrypt</Tab>
          <Tab value="decrypt">Decrypt</Tab>
          <Tab value="batch">Batch</Tab>
          <Tab value="settings">Settings</Tab>
          <Tab value="help">Help</Tab>
        </TabList>
      </Tabs>
      <div class="header-actions">
        <ChangelogAction />
        <span v-if="version" class="version-display">v{{ version }}</span>
      </div>
    </div>

    <!-- Tab Content Area -->
    <div class="tab-panels">
      <div v-show="activeTab === 'encrypt'" class="tab-panel">
        <EncryptTab />
      </div>
      <div v-show="activeTab === 'decrypt'" class="tab-panel">
        <DecryptTab />
      </div>
      <div v-show="activeTab === 'batch'" class="tab-panel">
        <BatchTab />
      </div>
      <div v-show="activeTab === 'settings'" class="tab-panel">
        <SettingsTab />
      </div>
      <div v-show="activeTab === 'help'" class="tab-panel">
        <HelpTab />
      </div>
    </div>

    <!-- Mobile Bottom Navigation (shown only on iOS/Android, waits for platform detection) -->
    <BottomNav
      v-if="isInitialized && isMobile"
      :active-tab="activeTab"
      @switch-tab="switchTab"
    />
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
  --border-strong: #c2c2c2;   /* High-contrast borders */
  --drop-border: #b0b0b0;     /* Drag/drop target border */
  --drop-border-active: #2f7ee6;
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
  --border-strong: #5a5a5a;
  --drop-border: #6a6a6a;
  --drop-border-active: #5fb0ff;
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
  height: 100dvh; /* Dynamic viewport height for mobile */
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
  height: 100%;
  background: var(--bg);
  overflow: hidden;
}

/* Desktop Header with Tabs and Version */
.desktop-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  background: var(--panel);
  border-bottom: 1px solid var(--border);
  padding-right: 16px;
  flex-shrink: 0;
  flex-grow: 0;
}

/* Desktop Tab Navigation */
.desktop-tabs {
  padding: 0;
  flex: 1;
  min-width: 0;
}

.header-actions {
  display: flex;
  align-items: center;
  gap: 8px;
}

/* Version Display */
.version-display {
  font-size: 1rem;
  color: var(--muted);
  white-space: nowrap;
  user-select: none;
}

/* Tab Content Area */
.tab-panels {
  flex: 1;
  min-height: 0; /* Required for flex item to respect overflow */
  display: flex;
  flex-direction: column;
  background: var(--bg);
}

.tab-panel {
  display: flex;
  flex-direction: column;
  flex: 1;
  min-height: 0;
}

/* PrimeVue Tabs customization */
.desktop-tabs :deep(.p-tablist-tab-list) {
  border-width: 0;
  background: transparent;
}

.desktop-tabs :deep(.p-tablist-tab-list) {
  padding-left: 16px;
}

.desktop-tabs :deep(.p-tab) {
  font-weight: 600;
}

</style>
