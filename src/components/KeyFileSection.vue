<!-- components/KeyFileSection.vue - Key File Input Section -->
<!--
  Provides a key file picker with Browse and Generate buttons.
  Used by EncryptTab, DecryptTab, and BatchTab for optional two-factor encryption.

  Props:
  - modelValue: The key file path (v-model)
  - disabled: Whether the inputs are disabled (during processing)

  Emits:
  - update:modelValue: When the key file path changes
-->

<script setup lang="ts">
import { NButton, NInput } from 'naive-ui';
import { useTauri } from '../composables/useTauri';

withDefaults(
  defineProps<{
    modelValue: string;
    disabled: boolean;
    showGenerate?: boolean;
  }>(),
  { showGenerate: true },
);

const emit = defineEmits<{
  'update:modelValue': [value: string];
}>();

const tauri = useTauri();

async function handleBrowse() {
  const path = await tauri.selectFile('Select Key File', [
    { name: 'Key Files', extensions: ['key'] },
    { name: 'All Files', extensions: ['*'] },
  ]);
  if (path) {
    emit('update:modelValue', path);
  }
}

async function handleGenerate() {
  const path = await tauri.selectSavePath('Save Key File', undefined, [
    { name: 'Key Files', extensions: ['key'] },
  ]);
  if (path) {
    await tauri.generateKeyFile(path);
    emit('update:modelValue', path);
  }
}

function handleClear() {
  emit('update:modelValue', '');
}
</script>

<template>
  <div class="form-group">
    <label>Key File (optional):</label>
    <div class="file-input-group">
      <NInput
        :value="modelValue"
        readonly
        placeholder="No key file selected"
        :disabled="disabled"
      />
      <NButton
        v-if="modelValue"
        type="primary"
        @click="handleClear"
        :disabled="disabled"
        title="Remove key file"
      >
        Clear
      </NButton>
      <NButton
        type="primary"
        @click="handleBrowse"
        :disabled="disabled"
        title="Select an existing key file"
      >
        Browse
      </NButton>
      <NButton
        v-if="showGenerate"
        type="primary"
        @click="handleGenerate"
        :disabled="disabled"
        title="Generate a new random key file"
      >
        Generate
      </NButton>
    </div>
    <p class="hint-text">
      Optional two-factor protection. The same key file is required for decryption.
    </p>
  </div>
</template>
