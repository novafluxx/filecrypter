<!-- BottomNav.vue - Mobile Bottom Navigation Component -->
<!--
  iOS/Android-style bottom navigation bar for mobile platforms.
  This component is conditionally rendered only on iOS and Android
  (controlled by usePlatform composable in App.vue).

  Features:
  - 5 tab items with SVG icons and text labels
  - Visual active state highlighting using accent color
  - Safe area padding for notched devices (iPhone X+, etc.)
  - Touch-friendly tap targets (min 56px width)

  Props:
  - activeTab: Currently selected tab identifier

  Events:
  - switch-tab: Emitted when user taps a navigation item

  Usage:
    <BottomNav
      :active-tab="activeTab"
      @switch-tab="switchTab"
    />
-->

<script setup lang="ts">
// Tab identifier type - must match the tabs defined in App.vue
type TabName = 'encrypt' | 'decrypt' | 'batch' | 'settings' | 'help';

// Props: receives the currently active tab from parent
defineProps<{
  activeTab: TabName;
}>();

// Events: emits tab switch requests to parent
const emit = defineEmits<{
  (e: 'switch-tab', tab: TabName): void;
}>();

// Navigation tab configuration
// Each tab has an id (used for routing) and a display label
const tabs: { id: TabName; label: string }[] = [
  { id: 'encrypt', label: 'Encrypt' },
  { id: 'decrypt', label: 'Decrypt' },
  { id: 'batch', label: 'Batch' },
  { id: 'settings', label: 'Settings' },
  { id: 'help', label: 'Help' },
];
</script>

<template>
  <nav class="bottom-nav">
    <button
      v-for="tab in tabs"
      :key="tab.id"
      class="nav-item"
      :class="{ active: activeTab === tab.id }"
      @click="emit('switch-tab', tab.id)"
    >
      <!-- Encrypt Icon (Lock) -->
      <svg v-if="tab.id === 'encrypt'" class="nav-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
        <rect x="3" y="11" width="18" height="11" rx="2" ry="2"/>
        <path d="M7 11V7a5 5 0 0 1 10 0v4"/>
      </svg>

      <!-- Decrypt Icon (Unlock) -->
      <svg v-if="tab.id === 'decrypt'" class="nav-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
        <rect x="3" y="11" width="18" height="11" rx="2" ry="2"/>
        <path d="M7 11V7a5 5 0 0 1 9.9-1"/>
      </svg>

      <!-- Batch Icon (Stack) -->
      <svg v-if="tab.id === 'batch'" class="nav-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
        <rect x="2" y="7" width="16" height="14" rx="2"/>
        <path d="M6 3h12a2 2 0 0 1 2 2v2"/>
        <path d="M22 11v8a2 2 0 0 1-2 2"/>
      </svg>

      <!-- Settings Icon (Gear) -->
      <svg v-if="tab.id === 'settings'" class="nav-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
        <circle cx="12" cy="12" r="3"/>
        <path d="M12 1v2M12 21v2M4.22 4.22l1.42 1.42M18.36 18.36l1.42 1.42M1 12h2M21 12h2M4.22 19.78l1.42-1.42M18.36 5.64l1.42-1.42"/>
      </svg>

      <!-- Help Icon (Question Circle) -->
      <svg v-if="tab.id === 'help'" class="nav-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
        <circle cx="12" cy="12" r="10"/>
        <path d="M9.09 9a3 3 0 0 1 5.83 1c0 2-3 3-3 3"/>
        <line x1="12" y1="17" x2="12.01" y2="17"/>
      </svg>

      <span class="nav-label">{{ tab.label }}</span>
    </button>
  </nav>
</template>

<style scoped>
/*
 * Bottom Navigation Styles
 *
 * Designed to match iOS/Android native tab bar conventions:
 * - Fixed at bottom of screen (via parent flexbox layout)
 * - Icons above labels
 * - Safe area handling for notched devices
 */

.bottom-nav {
  display: flex;
  justify-content: space-around;
  align-items: center;
  background: var(--panel);
  border-top: 1px solid var(--border);
  padding: 8px 4px;
  /* Safe area inset for notched devices (iPhone X and later)
     Adds extra padding at the bottom to avoid the home indicator */
  padding-bottom: calc(8px + env(safe-area-inset-bottom, 0px));
  /* Prevent nav from shrinking when content is tall */
  flex-shrink: 0;
}

.nav-item {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 4px;
  padding: 6px 8px;
  background: transparent;
  border: none;
  cursor: pointer;
  color: var(--muted);
  transition: color 0.15s;
  font-family: inherit;
  /* Minimum touch target size for accessibility (Apple HIG recommends 44pt) */
  min-width: 56px;
  border-radius: 8px;
}

/* Tap feedback - shows subtle background on press */
.nav-item:active {
  background: var(--panel-alt);
}

/* Active tab state - highlighted with accent color */
.nav-item.active {
  color: var(--accent);
}

/* Icon sizing - 24x24 is standard for mobile nav icons */
.nav-icon {
  width: 24px;
  height: 24px;
}

/* Label styling - small text below icon */
.nav-label {
  font-size: 11px;
  font-weight: 500;
  white-space: nowrap;
}
</style>
