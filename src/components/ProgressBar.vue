<!-- components/ProgressBar.vue - Animated Progress Bar -->
<!--
  Displays an animated progress bar with percentage and status message.

  Props:
  - percent: Current progress (0-100)
  - message: Status message to display below the bar
-->

<script setup lang="ts">
import ProgressBar from 'primevue/progressbar';

defineProps<{
  percent: number;
  message: string;
}>();
</script>

<template>
  <div class="progress-container">
    <ProgressBar
      :value="percent"
      :showValue="false"
      class="progress-bar"
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

.progress-bar {
  height: 6px;
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
