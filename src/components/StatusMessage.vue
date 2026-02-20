<!-- components/StatusMessage.vue - Status Message Display -->
<!--
  Displays status messages using PrimeVue's Message component.

  Props:
  - message: The message text to display
  - type: Message type ('success' | 'error' | 'info')
-->

<script setup lang="ts">
import Message from 'primevue/message';
import type { StatusType } from '../types/crypto';

defineProps<{
  message: string;
  type: StatusType;
}>();

// Map app status types to PrimeVue severity values
function toSeverity(type: StatusType): 'success' | 'error' | 'info' {
  return type;
}
</script>

<template>
  <Message
    class="status-message"
    :severity="toSeverity(type)"
    :closable="false"
    role="status"
    aria-live="polite"
  >
    <span class="selectable">{{ message }}</span>
  </Message>
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
