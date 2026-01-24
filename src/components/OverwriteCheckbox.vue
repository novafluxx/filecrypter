<!-- components/OverwriteCheckbox.vue - Shared Overwrite Safety Checkbox -->
<!--
  Provides a checkbox to enable "never overwrite" mode with auto-renaming.
  Used by both EncryptTab and DecryptTab.

  Props:
  - modelValue: Whether never-overwrite mode is enabled
  - disabled: Whether the checkbox is disabled (during processing)
-->

<script setup lang="ts">
import { NCheckbox } from 'naive-ui';

defineProps<{
  modelValue: boolean;
  disabled: boolean;
}>();

defineEmits<{
  'update:modelValue': [value: boolean];
}>();
</script>

<template>
  <div class="form-group">
    <NCheckbox
      :checked="modelValue"
      @update:checked="$emit('update:modelValue', $event)"
      :disabled="disabled"
    >
      Never overwrite existing files (auto-rename on conflicts)
    </NCheckbox>
    <p class="hint-text">
      If the output name already exists, we'll save as "name (1)".
    </p>
  </div>
</template>
