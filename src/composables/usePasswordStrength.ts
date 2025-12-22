// composables/usePasswordStrength.ts - Password Strength Analysis
//
// This composable analyzes password quality and provides reactive feedback.
// Uses a simple scoring system without external dependencies.
//
// Scoring criteria:
// - Length: 8+ chars (base), 12+ (bonus), 16+ (extra bonus)
// - Character variety: lowercase, uppercase, numbers, symbols
// - Penalties: sequential chars, repeated chars, common patterns

import { computed, type Ref } from 'vue';

/**
 * Strength levels for visual feedback
 * - weak: Password is easily crackable
 * - fair: Meets minimum requirements but could be stronger
 * - good: Reasonably secure for most purposes
 * - strong: Excellent password strength
 */
export type StrengthLevel = 'weak' | 'fair' | 'good' | 'strong';

/**
 * Password strength analysis result
 */
export interface PasswordStrength {
  /** Score from 0-100 */
  score: number;
  /** Categorical strength level */
  level: StrengthLevel;
  /** Suggestions for improvement */
  feedback: string[];
}

/**
 * Analyze password strength reactively
 *
 * @param password - Reactive ref containing the password string
 * @returns Object containing reactive strength analysis
 *
 * @example
 * ```ts
 * const password = ref('');
 * const { strength } = usePasswordStrength(password);
 *
 * // In template:
 * // {{ strength.level }} - {{ strength.score }}%
 * ```
 */
export function usePasswordStrength(password: Ref<string>) {
  const strength = computed<PasswordStrength>(() => {
    const pwd = password.value;
    let score = 0;
    const feedback: string[] = [];

    // Empty password
    if (pwd.length === 0) {
      return { score: 0, level: 'weak', feedback: [] };
    }

    // === Length scoring ===
    // Base requirement: 8 characters
    if (pwd.length >= 8) {
      score += 25;
    } else {
      feedback.push('Use at least 8 characters');
    }

    // Bonus for longer passwords
    if (pwd.length >= 12) {
      score += 15;
    }
    if (pwd.length >= 16) {
      score += 10;
    }

    // === Character variety scoring ===
    const hasLowercase = /[a-z]/.test(pwd);
    const hasUppercase = /[A-Z]/.test(pwd);
    const hasNumbers = /[0-9]/.test(pwd);
    const hasSymbols = /[^a-zA-Z0-9]/.test(pwd);

    if (hasLowercase) score += 10;
    if (hasUppercase) score += 15;
    if (hasNumbers) score += 15;
    if (hasSymbols) score += 15;

    // Feedback for missing character types
    if (!hasUppercase && pwd.length >= 8) {
      feedback.push('Add uppercase letters');
    }
    if (!hasNumbers && pwd.length >= 8) {
      feedback.push('Add numbers');
    }
    if (!hasSymbols && pwd.length >= 8) {
      feedback.push('Add special characters (!@#$%...)');
    }

    // === Pattern penalties ===
    // Repeated characters (e.g., "aaa", "111")
    if (/(.)\1{2,}/.test(pwd)) {
      score -= 10;
      feedback.push('Avoid repeated characters');
    }

    // Sequential patterns
    if (/123|234|345|456|567|678|789|890/.test(pwd)) {
      score -= 10;
      feedback.push('Avoid sequential numbers');
    }

    if (/abc|bcd|cde|def|efg|fgh|ghi|hij|ijk|jkl|klm|lmn|mno|nop|opq|pqr|qrs|rst|stu|tuv|uvw|vwx|wxy|xyz/i.test(pwd)) {
      score -= 10;
      feedback.push('Avoid sequential letters');
    }

    // Common weak patterns
    if (/qwerty|asdf|zxcv|password|123456|admin|letmein/i.test(pwd)) {
      score -= 20;
      feedback.push('Avoid common patterns');
    }

    // === Clamp score to 0-100 ===
    score = Math.max(0, Math.min(100, score));

    // === Determine strength level ===
    let level: StrengthLevel;
    if (score < 30) {
      level = 'weak';
    } else if (score < 50) {
      level = 'fair';
    } else if (score < 75) {
      level = 'good';
    } else {
      level = 'strong';
    }

    return { score, level, feedback };
  });

  return { strength };
}
