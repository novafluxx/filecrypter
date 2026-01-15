<!-- components/ProgressBar.vue - Animated Progress Bar -->
<!--
  Displays an animated progress bar with percentage and status message.

  Props:
  - percent: Current progress (0-100)
  - message: Status message to display below the bar

  Uses Naive UI's NProgress component for the actual progress bar.
-->

<script setup lang="ts">
import { NProgress } from 'naive-ui';

defineProps<{
  percent: number;
  message: string;
}>();
</script>

<template>
  <div class="progress-container">
    <!-- Naive UI Progress bar -->
    <NProgress
      type="line"
      :percentage="percent"
      :show-indicator="false"
      :height="6"
      :border-radius="3"
    />

    <!-- Progress info -->
    <div class="progress-info">
      <span class="progress-message">{{ message }}</span>
      <span class="progress-percent">{{ percent }}%</span>
    </div>
  </div>
</template>

<style scoped>
.progress-container {
  margin: 16px 0;
  padding: 12px;
  background: var(--panel-alt);
  border-radius: 6px;
  border: 1px solid var(--border);
}

.progress-info {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-top: 8px;
}

.progress-message {
  font-size: 16px;
  color: var(--muted);
}

.progress-percent {
  font-size: 16px;
  font-weight: 600;
  color: var(--accent);
}
</style>
