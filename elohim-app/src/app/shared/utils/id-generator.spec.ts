import { generateId, generateMapId, generateExtensionId, generateNegotiationId } from './id-generator';

describe('id-generator utils', () => {
  describe('generateId', () => {
    it('should generate ID with correct prefix', () => {
      const id = generateId('test');
      expect(id).toMatch(/^test-\d+-[a-z0-9]+$/);
    });

    it('should generate unique IDs', () => {
      const id1 = generateId('test');
      const id2 = generateId('test');
      expect(id1).not.toBe(id2);
    });

    it('should handle different prefixes', () => {
      const id1 = generateId('map');
      const id2 = generateId('ext');
      expect(id1).toContain('map-');
      expect(id2).toContain('ext-');
    });
  });

  describe('generateMapId', () => {
    it('should generate domain map ID', () => {
      const id = generateMapId('domain');
      expect(id).toMatch(/^map-domain-\d+-[a-z0-9]+$/);
    });

    it('should generate person map ID', () => {
      const id = generateMapId('person');
      expect(id).toMatch(/^map-person-\d+-[a-z0-9]+$/);
    });

    it('should generate collective map ID', () => {
      const id = generateMapId('collective');
      expect(id).toMatch(/^map-collective-\d+-[a-z0-9]+$/);
    });

    it('should generate unique map IDs', () => {
      const id1 = generateMapId('domain');
      const id2 = generateMapId('domain');
      expect(id1).not.toBe(id2);
    });
  });

  describe('generateExtensionId', () => {
    it('should generate extension ID', () => {
      const id = generateExtensionId();
      expect(id).toMatch(/^ext-\d+-[a-z0-9]+$/);
    });

    it('should generate unique extension IDs', () => {
      const id1 = generateExtensionId();
      const id2 = generateExtensionId();
      expect(id1).not.toBe(id2);
    });
  });

  describe('generateNegotiationId', () => {
    it('should generate negotiation ID', () => {
      const id = generateNegotiationId();
      expect(id).toMatch(/^nego-\d+-[a-z0-9]+$/);
    });

    it('should generate unique negotiation IDs', () => {
      const id1 = generateNegotiationId();
      const id2 = generateNegotiationId();
      expect(id1).not.toBe(id2);
    });
  });
});
