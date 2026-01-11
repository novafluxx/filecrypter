// composables/usePasswordVisibility.ts - Password Visibility Toggle
//
// This composable provides a simple toggle for showing/hiding password fields.
// Used in EncryptTab, DecryptTab, and BatchTab components.

import { ref } from 'vue';

/**
 * Composable for managing password visibility toggle
 *
 * Provides reactive state and a toggle function for controlling
 * whether password input fields show text or dots.
 *
 * @returns Object containing visibility state and toggle function
 *
 * @example
 * ```ts
 * const { isPasswordVisible, togglePasswordVisibility } = usePasswordVisibility();
 *
 * // In template:
 * // <input :type="isPasswordVisible ? 'text' : 'password'" />
 * // <button @click="togglePasswordVisibility">Show/Hide</button>
 * ```
 */
export function usePasswordVisibility() {
  const isPasswordVisible = ref(false);

  /**
   * Toggles password visibility between hidden and visible
   */
  function togglePasswordVisibility() {
    isPasswordVisible.value = !isPasswordVisible.value;
  }

  return {
    isPasswordVisible,
    togglePasswordVisibility,
  };
}
