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
import Button from 'primevue/button';
import ButtonGroup from 'primevue/buttongroup';
import Checkbox from 'primevue/checkbox';
import InputText from 'primevue/inputtext';
import { open } from '@tauri-apps/plugin-dialog';
import { useSettings, type ThemeMode } from '../composables/useSettings';

// Initialize settings composable
const settings = useSettings();

// Computed for cleaner template bindings
const currentTheme = computed(() => settings.theme.value);

const compressionEnabled = computed({
  get: () => settings.defaultCompression.value,
  set: (v: boolean) => settings.setDefaultCompression(v),
});

const neverOverwrite = computed({
  get: () => settings.defaultNeverOverwrite.value,
  set: (v: boolean) => settings.setDefaultNeverOverwrite(v),
});

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
          <ButtonGroup>
            <Button
              :severity="currentTheme === 'light' ? undefined : 'secondary'"
              :outlined="currentTheme !== 'light'"
              @click="handleThemeChange('light')"
              label="Light"
            />
            <Button
              :severity="currentTheme === 'dark' ? undefined : 'secondary'"
              :outlined="currentTheme !== 'dark'"
              @click="handleThemeChange('dark')"
              label="Dark"
            />
            <Button
              :severity="currentTheme === 'system' ? undefined : 'secondary'"
              :outlined="currentTheme !== 'system'"
              @click="handleThemeChange('system')"
              label="System"
            />
          </ButtonGroup>
        </div>
      </section>

      <!-- Encryption Defaults Section -->
      <section class="settings-section">
        <h2 class="section-title">Encryption Defaults</h2>

        <div class="form-group">
          <div class="checkbox-field">
            <Checkbox
              v-model="compressionEnabled"
              :binary="true"
              inputId="settings-compression"
            />
            <label for="settings-compression">Enable compression by default</label>
          </div>
          <p class="hint-text">
            Single file encryption only. Batch mode always uses compression.
          </p>
        </div>

        <div class="form-group">
          <div class="checkbox-field">
            <Checkbox
              v-model="neverOverwrite"
              :binary="true"
              inputId="settings-overwrite"
            />
            <label for="settings-overwrite">Never overwrite existing files by default</label>
          </div>
          <p class="hint-text">
            Auto-rename to "name (1)" on conflicts.
          </p>
        </div>

        <div class="form-group">
          <label class="setting-label">Default Output Directory:</label>
          <div class="file-input-group">
            <InputText
              :modelValue="outputDirectory || ''"
              readonly
              placeholder="Same as input file (default)"
              fluid
            />
            <Button
              v-if="outputDirectory"
              @click="handleClearOutputDir"
              title="Clear default directory"
              severity="secondary"
              label="Clear"
            />
            <Button
              @click="handleSelectOutputDir"
              title="Choose a default folder for encrypted/decrypted files"
              label="Browse"
            />
          </div>
          <p class="hint-text">
            Leave empty to save files alongside the originals.
          </p>
        </div>
      </section>

      <!-- Reset Section -->
      <section class="settings-section reset-section">
        <Button
          @click="handleResetToDefaults"
          title="Restore all settings to their original values"
          outlined
          label="Reset to Defaults"
        />
      </section>
    </div>
  </div>
</template>

<style scoped>
/* Component-specific styles - shared styles are in src/shared.css */

.tab-content {
  padding: 16px;
  max-width: 800px;
  width: 100%;
  margin: 0 auto;
  flex: 1;
  display: flex;
  flex-direction: column;
  min-height: 0;
}

.content-panel {
  background: var(--panel);
  border: 1px solid var(--border);
  border-radius: 8px;
  padding: 16px;
  position: relative;
  flex: 1;
  display: flex;
  flex-direction: column;
  overflow-y: auto;
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

/* Reset Section */
.reset-section {
  text-align: center;
}
</style>
