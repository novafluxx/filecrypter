<!-- components/PasswordSection.vue - Shared Password Input Section -->
<!--
  Provides a password input field with optional strength meter or hint text.
  Used by both EncryptTab (with strength meter) and DecryptTab (with hint).

  Props:
  - inputId: ID for the input element (for label association)
  - modelValue: The password value (v-model)
  - placeholder: Input placeholder text
  - disabled: Whether the input is disabled (during processing)
  - showStrengthMeter: If true, shows password strength meter (encrypt mode)
  - strength: Password strength object (required when showStrengthMeter is true)
  - isPasswordValid: Whether password meets minimum requirements
  - hintText: Optional hint text shown when password is empty (decrypt mode)
-->

<script setup lang="ts">
import { NInput } from 'naive-ui';
import PasswordStrengthMeter from './PasswordStrengthMeter.vue';
import type { PasswordStrength } from '../composables/usePasswordStrength';

defineProps<{
  inputId: string;
  modelValue: string;
  placeholder: string;
  disabled: boolean;
  autocomplete: 'new-password' | 'current-password';
  showStrengthMeter?: boolean;
  strength?: PasswordStrength;
  isPasswordValid?: boolean;
  hintText?: string;
}>();

defineEmits<{
  'update:modelValue': [value: string];
}>();
</script>

<template>
  <div class="form-group">
    <label :for="inputId">Password:</label>
    <NInput
      :input-props="{
        id: inputId,
        autocomplete,
        spellcheck: 'false',
        autocapitalize: 'off',
        autocorrect: 'off'
      }"
      type="password"
      show-password-on="click"
      :value="modelValue"
      @update:value="$emit('update:modelValue', $event)"
      :placeholder="placeholder"
      :disabled="disabled"
    />
    <!-- Password strength meter (encrypt mode) -->
    <PasswordStrengthMeter
      v-if="showStrengthMeter && strength && modelValue.length > 0"
      :strength="strength"
      :show-feedback="!isPasswordValid"
    />
    <!-- Hint text (decrypt mode) -->
    <p v-else-if="hintText && modelValue.length === 0" class="hint-text">
      {{ hintText }}
    </p>
  </div>
</template>
