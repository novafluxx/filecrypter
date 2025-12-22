<!-- components/ProgressBar.vue - Animated Progress Bar -->
<!--
  Displays an animated progress bar with percentage and status message.

  Props:
  - percent: Current progress (0-100)
  - message: Status message to display below the bar

  The bar features a gradient fill and smooth animation on progress changes.
-->

<script setup lang="ts">
import { computed } from 'vue';

const props = defineProps<{
  percent: number;
  message: string;
}>();

// Compute bar width, clamping to 0-100
const barWidth = computed(() => `${Math.min(100, Math.max(0, props.percent))}%`);
</script>

<template>
  <div class="progress-container">
    <!-- Progress bar track -->
    <div class="progress-bar-bg">
      <div class="progress-bar-fill" :style="{ width: barWidth }"></div>
    </div>

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
  background: var(--bg-tertiary);
  border-radius: 8px;
  border: 1px solid var(--border-color);
}

.progress-bar-bg {
  height: 8px;
  background: var(--border-color);
  border-radius: 4px;
  overflow: hidden;
}

.progress-bar-fill {
  height: 100%;
  background: linear-gradient(90deg, var(--accent-primary), var(--accent-secondary));
  border-radius: 4px;
  transition: width 0.3s ease;
}

.progress-info {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-top: 8px;
}

.progress-message {
  font-size: 13px;
  color: var(--text-secondary);
}

.progress-percent {
  font-size: 13px;
  font-weight: 600;
  color: var(--accent-primary);
}
</style>
