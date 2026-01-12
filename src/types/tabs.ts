// tabs.ts - Tab Navigation Types
//
// Shared type definitions for tab navigation used by App.vue and BottomNav.vue.
// Single source of truth to ensure type consistency across components.

/**
 * Valid tab identifiers for the application navigation.
 * Used for both desktop top tabs and mobile bottom navigation.
 */
export type TabName = 'encrypt' | 'decrypt' | 'batch' | 'settings' | 'help';
