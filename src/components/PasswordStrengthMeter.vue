<!-- components/PasswordStrengthMeter.vue - Visual Password Strength Indicator -->
<!--
  Displays a color-coded strength bar with optional feedback suggestions.

  Props:
  - strength: PasswordStrength object from usePasswordStrength composable
  - showFeedback: Whether to display improvement suggestions (default: true)

  The bar color transitions smoothly as the password strength changes.
-->

<script setup lang="ts">
import { computed } from 'vue';
import type { PasswordStrength } from '../composables/usePasswordStrength';

const props = withDefaults(defineProps<{
  strength: PasswordStrength;
  showFeedback?: boolean;
}>(), {
  showFeedback: true
});

// Compute bar width based on score
const barWidth = computed(() => `${props.strength.score}%`);

// Color mapping for each strength level
const levelColors: Record<string, string> = {
  weak: '#dc3545',    // Red
  fair: '#ffc107',    // Yellow/Orange
  good: '#28a745',    // Green
  strong: '#20c997'   // Teal
};

const barColor = computed(() => levelColors[props.strength.level]);

// Format level for display (capitalize first letter)
const levelDisplay = computed(() =>
  props.strength.level.charAt(0).toUpperCase() + props.strength.level.slice(1)
);
</script>

<template>
  <div class="password-strength">
    <!-- Strength bar -->
    <div class="strength-bar-container">
      <div
        class="strength-bar"
        :style="{ width: barWidth, backgroundColor: barColor }"
      ></div>
    </div>

    <!-- Strength label -->
    <span class="strength-label" :class="strength.level">
      {{ levelDisplay }}
    </span>

    <!-- Feedback suggestions -->
    <ul
      v-if="showFeedback && strength.feedback.length > 0"
      class="strength-feedback"
    >
      <li v-for="tip in strength.feedback" :key="tip">{{ tip }}</li>
    </ul>
  </div>
</template>

<style scoped>
.password-strength {
  margin-top: 8px;
}

.strength-bar-container {
  height: 4px;
  background: var(--border-color);
  border-radius: 2px;
  overflow: hidden;
}

.strength-bar {
  height: 100%;
  border-radius: 2px;
  transition: width 0.3s ease, background-color 0.3s ease;
}

.strength-label {
  font-size: 12px;
  font-weight: 500;
  margin-top: 4px;
  display: inline-block;
}

/* Level-specific colors for the label (kept static for clarity) */
.strength-label.weak {
  color: #dc3545;
}

.strength-label.fair {
  color: #d4a106;
}

.strength-label.good {
  color: #28a745;
}

.strength-label.strong {
  color: #20c997;
}

.strength-feedback {
  margin-top: 6px;
  padding-left: 18px;
  font-size: 11px;
  color: var(--text-secondary);
  list-style-type: disc;
}

.strength-feedback li {
  margin-bottom: 2px;
}
</style>
