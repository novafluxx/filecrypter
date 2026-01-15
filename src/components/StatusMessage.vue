<!-- components/StatusMessage.vue - Status Message Display -->
<!--
  Displays status messages using Naive UI's NAlert component.

  Props:
  - message: The message text to display
  - type: Message type ('success' | 'error' | 'info')

  Replaces the custom .status-message CSS with Naive UI's NAlert.
-->

<script setup lang="ts">
import { NAlert } from 'naive-ui';
import type { StatusType } from '../types/crypto';

defineProps<{
  message: string;
  type: StatusType;
}>();
</script>

<template>
  <NAlert
    class="status-message"
    :type="type"
    :show-icon="true"
    role="status"
    aria-live="polite"
  >
    <span class="selectable">{{ message }}</span>
  </NAlert>
</template>

<style scoped>
.status-message {
  margin-top: 16px;
}

.selectable {
  user-select: text;
  -webkit-user-select: text;
}
</style>
