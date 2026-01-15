<!-- components/SettingsTab.vue - Application Settings Interface -->
<!--
  This component provides the UI for configuring application settings.

  Features:
  - Theme selection (Light/Dark/System)
  - Default encryption options
  - Default output directory
  - Reset to defaults
  - Auto-save (no save button needed)
-->

<script setup lang="ts">
import { computed } from 'vue';
import { NButton, NCheckbox, NInput } from 'naive-ui';
import { open } from '@tauri-apps/plugin-dialog';
import { useSettings, type ThemeMode } from '../composables/useSettings';

// Initialize settings composable
const settings = useSettings();

// Computed for cleaner template bindings
const currentTheme = computed(() => settings.theme.value);
const compressionEnabled = computed(() => settings.defaultCompression.value);
const neverOverwrite = computed(() => settings.defaultNeverOverwrite.value);
const outputDirectory = computed(() => settings.defaultOutputDirectory.value);

/**
 * Handle theme selection
 */
async function handleThemeChange(newTheme: ThemeMode) {
  await settings.setTheme(newTheme);
}

/**
 * Select output directory
 */
async function handleSelectOutputDir() {
  const dir = await open({
    title: 'Select Default Output Directory',
    directory: true,
    multiple: false,
  });

  if (dir) {
    await settings.setDefaultOutputDirectory(dir as string);
  }
}

/**
 * Clear default output directory
 */
async function handleClearOutputDir() {
  await settings.setDefaultOutputDirectory(null);
}

/**
 * Reset all settings to defaults
 */
async function handleResetToDefaults() {
  await settings.resetToDefaults();
}
</script>

<template>
  <div class="tab-content">
    <div class="content-panel">
      <!-- Appearance Section -->
      <section class="settings-section">
        <h2 class="section-title">Appearance</h2>

        <div class="form-group">
          <label class="setting-label">Theme:</label>
          <div class="theme-toggle">
            <button
              class="theme-btn"
              :class="{ active: currentTheme === 'light' }"
              @click="handleThemeChange('light')"
              title="Use light color scheme"
            >
              Light
            </button>
            <button
              class="theme-btn"
              :class="{ active: currentTheme === 'dark' }"
              @click="handleThemeChange('dark')"
              title="Use dark color scheme"
            >
              Dark
            </button>
            <button
              class="theme-btn"
              :class="{ active: currentTheme === 'system' }"
              @click="handleThemeChange('system')"
              title="Follow your operating system's color scheme"
            >
              System
            </button>
          </div>
        </div>
      </section>

      <!-- Encryption Defaults Section -->
      <section class="settings-section">
        <h2 class="section-title">Encryption Defaults</h2>

        <div class="form-group">
          <NCheckbox
            :checked="compressionEnabled"
            @update:checked="v => settings.setDefaultCompression(v)"
          >
            Enable compression by default
          </NCheckbox>
          <p class="hint-text">
            Single file encryption only. Batch mode always uses compression.
          </p>
        </div>

        <div class="form-group">
          <NCheckbox
            :checked="neverOverwrite"
            @update:checked="v => settings.setDefaultNeverOverwrite(v)"
          >
            Never overwrite existing files by default
          </NCheckbox>
          <p class="hint-text">
            Auto-rename to "name (1)" on conflicts.
          </p>
        </div>

        <div class="form-group">
          <label class="setting-label">Default Output Directory:</label>
          <div class="file-input-group">
            <NInput
              :value="outputDirectory || ''"
              readonly
              placeholder="Same as input file (default)"
            />
            <NButton
              v-if="outputDirectory"
              @click="handleClearOutputDir"
              title="Clear default directory"
            >
              Clear
            </NButton>
            <NButton
              type="primary"
              @click="handleSelectOutputDir"
              title="Choose a default folder for encrypted/decrypted files"
            >
              Browse
            </NButton>
          </div>
          <p class="hint-text">
            Leave empty to save files alongside the originals.
          </p>
        </div>
      </section>

      <!-- Reset Section -->
      <section class="settings-section reset-section">
        <NButton
          @click="handleResetToDefaults"
          title="Restore all settings to their original values"
        >
          Reset to Defaults
        </NButton>
      </section>
    </div>
  </div>
</template>

<style scoped>
/* Component-specific styles - shared styles are in src/shared.css */

.tab-content {
  padding: 16px;
  max-width: 800px;
  margin: 0 auto;
  position: relative;
}

.content-panel {
  background: var(--panel);
  border: 1px solid var(--border);
  border-radius: 8px;
  padding: 16px;
  position: relative;
}

/* Settings Sections */
.settings-section {
  margin-bottom: 16px;
  padding-bottom: 16px;
  border-bottom: 1px solid var(--border);
}

.settings-section:last-child {
  margin-bottom: 0;
  padding-bottom: 0;
  border-bottom: none;
}

.section-title {
  font-size: 17px;
  font-weight: 600;
  color: var(--text);
  margin: 0 0 10px 0;
}

.setting-label {
  display: block;
  font-size: 16px;
  font-weight: 500;
  color: var(--text);
  margin-bottom: 6px;
}

/* Tighter form group spacing for settings */
.form-group {
  margin-bottom: 10px;
}

.form-group:last-child {
  margin-bottom: 0;
}

/* Theme Toggle (similar to BatchTab mode toggle) */
.theme-toggle {
  display: flex;
  gap: 4px;
  padding: 4px;
  background: var(--panel-alt);
  border-radius: 4px;
  border: 1px solid var(--border);
}

.theme-btn {
  flex: 1;
  padding: 6px 12px;
  border: none;
  border-radius: 4px;
  background: transparent;
  color: var(--muted);
  font-size: 16px;
  font-weight: 500;
  cursor: default;
  transition: all 0.15s;
  font-family: inherit;
}

.theme-btn:hover:not(.active) {
  color: var(--text);
  background: var(--border);
}

.theme-btn.active {
  background: var(--accent);
  color: white;
}

/* Reset Section */
.reset-section {
  text-align: center;
}
</style>
