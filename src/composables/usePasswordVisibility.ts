import { ref } from 'vue';

/**
 * Composable for managing password visibility toggle
 *
 * @returns {Object} Password visibility state and toggle function
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
