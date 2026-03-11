import { normalizeId } from './id-generator';

describe('id-generator', () => {
  describe('normalizeId', () => {
    it('should convert parts to lowercase kebab-case', () => {
      const result = normalizeId(['Epic', 'Name', 'Here']);
      expect(result).toBe('epic-name-here');
    });

    it('should replace non-alphanumeric characters with hyphens', () => {
      const result = normalizeId(['user_profile', 'ADMIN']);
      expect(result).toBe('user-profile-admin');
    });

    it('should handle mixed case and special characters', () => {
      const result = normalizeId(['Policy Maker', 'Epic!']);
      expect(result).toBe('policy-maker-epic');
    });

    it('should remove leading and trailing hyphens', () => {
      const result = normalizeId(['!start', 'middle', 'end!']);
      expect(result).toBe('start-middle-end');
    });

    it('should collapse multiple hyphens into one', () => {
      const result = normalizeId(['test---item', 'other__thing']);
      expect(result).toBe('test-item-other-thing');
    });

    it('should handle empty strings in parts', () => {
      const result = normalizeId(['', 'valid', '']);
      expect(result).toBe('valid');
    });

    it('should handle single part', () => {
      const result = normalizeId(['governance']);
      expect(result).toBe('governance');
    });

    it('should handle numbers in parts', () => {
      const result = normalizeId(['epic', '123', 'test']);
      expect(result).toBe('epic-123-test');
    });

    it('should handle parts with underscores', () => {
      const result = normalizeId(['policy_maker', 'user_type']);
      expect(result).toBe('policy-maker-user-type');
    });

    it('should handle parts with spaces', () => {
      const result = normalizeId(['My Epic Name']);
      expect(result).toBe('my-epic-name');
    });
  });
});
