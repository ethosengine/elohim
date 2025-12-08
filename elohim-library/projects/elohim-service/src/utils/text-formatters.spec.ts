import { titleCase, titleCaseWithSuffix, formatWithDelimiters } from './text-formatters';

describe('text-formatters', () => {
  describe('titleCase', () => {
    it('should convert underscore-separated text to title case', () => {
      const result = titleCase('policy_maker');
      expect(result).toBe('Policy Maker');
    });

    it('should handle single word', () => {
      const result = titleCase('governance');
      expect(result).toBe('Governance');
    });

    it('should handle hyphen separator', () => {
      const result = titleCase('governance-epic', '-');
      expect(result).toBe('Governance Epic');
    });

    it('should handle multiple underscores', () => {
      const result = titleCase('community_content_creator');
      expect(result).toBe('Community Content Creator');
    });

    it('should preserve existing capitals in middle of words', () => {
      const result = titleCase('userID');
      expect(result).toBe('UserID');
    });

    it('should handle empty string', () => {
      const result = titleCase('');
      expect(result).toBe('');
    });

    it('should handle mixed case input', () => {
      const result = titleCase('Policy_MAKER');
      expect(result).toBe('Policy MAKER');
    });
  });

  describe('titleCaseWithSuffix', () => {
    it('should add suffix to title cased text', () => {
      const result = titleCaseWithSuffix('governance', 'Epic');
      expect(result).toBe('Governance Epic');
    });

    it('should handle multiple words with suffix', () => {
      const result = titleCaseWithSuffix('policy_maker', 'Role');
      expect(result).toBe('Policy Maker Role');
    });

    it('should handle hyphen separator with suffix', () => {
      const result = titleCaseWithSuffix('social-medium', 'Domain', '-');
      expect(result).toBe('Social Medium Domain');
    });

    it('should handle empty suffix', () => {
      const result = titleCaseWithSuffix('test_case', '');
      expect(result).toBe('Test Case ');
    });

    it('should handle single character words', () => {
      const result = titleCaseWithSuffix('a_b_c', 'Test');
      expect(result).toBe('A B C Test');
    });
  });

  describe('formatWithDelimiters', () => {
    it('should handle hyphens and underscores', () => {
      const result = formatWithDelimiters('my-epic_name');
      expect(result).toBe('My Epic Name');
    });

    it('should handle only hyphens', () => {
      const result = formatWithDelimiters('kebab-case-text');
      expect(result).toBe('Kebab Case Text');
    });

    it('should handle only underscores', () => {
      const result = formatWithDelimiters('snake_case_text');
      expect(result).toBe('Snake Case Text');
    });

    it('should handle mixed delimiters', () => {
      const result = formatWithDelimiters('mixed-delimiter_text-here');
      expect(result).toBe('Mixed Delimiter Text Here');
    });

    it('should handle consecutive delimiters', () => {
      const result = formatWithDelimiters('test--double__underscore');
      expect(result).toBe('Test  Double  Underscore');
    });

    it('should handle single word', () => {
      const result = formatWithDelimiters('single');
      expect(result).toBe('Single');
    });

    it('should handle empty string', () => {
      const result = formatWithDelimiters('');
      expect(result).toBe('');
    });

    it('should handle leading and trailing delimiters', () => {
      const result = formatWithDelimiters('-test_name-');
      expect(result).toBe(' Test Name ');
    });

    it('should preserve numbers', () => {
      const result = formatWithDelimiters('test-123-name');
      expect(result).toBe('Test 123 Name');
    });

    it('should handle camelCase input', () => {
      const result = formatWithDelimiters('camelCaseText');
      expect(result).toBe('CamelCaseText');
    });
  });
});
