/**
 * Utility functions for text formatting
 */

/**
 * Convert text to Title Case
 * @param text - Text to convert
 * @param separator - Separator character (underscore or hyphen)
 * @returns Title cased text
 * @example
 * titleCase('policy_maker') // returns 'Policy Maker'
 * titleCase('governance-epic', '-') // returns 'Governance Epic'
 */
export function titleCase(text: string, separator: '_' | '-' = '_'): string {
  return text
    .split(separator)
    .map(word => word.charAt(0).toUpperCase() + word.slice(1))
    .join(' ');
}

/**
 * Convert text to Title Case and add a suffix
 * @param text - Text to convert
 * @param suffix - Suffix to add
 * @param separator - Separator character (underscore or hyphen)
 * @returns Title cased text with suffix
 * @example
 * titleCaseWithSuffix('policy_maker', 'Epic') // returns 'Policy Maker Epic'
 */
export function titleCaseWithSuffix(text: string, suffix: string, separator: '_' | '-' = '_'): string {
  return titleCase(text, separator) + ` ${suffix}`;
}

/**
 * Format text with multiple delimiters (hyphens and underscores)
 * @param text - Text to format
 * @returns Formatted text in Title Case
 * @example
 * formatWithDelimiters('my-epic_name') // returns 'My Epic Name'
 */
export function formatWithDelimiters(text: string): string {
  return text
    .replace(/[-_]/g, ' ')
    .split(' ')
    .map(word => word.charAt(0).toUpperCase() + word.slice(1))
    .join(' ');
}
