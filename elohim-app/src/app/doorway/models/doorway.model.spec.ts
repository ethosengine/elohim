import {
  reachLevelName,
  statusColor,
  tierColor,
  NodeStatus,
  StewardTier,
} from './doorway.model';

describe('Doorway Model Helpers', () => {
  describe('reachLevelName()', () => {
    it('should return "Private" for level 0', () => {
      expect(reachLevelName(0)).toBe('Private');
    });

    it('should return "Invited" for level 1', () => {
      expect(reachLevelName(1)).toBe('Invited');
    });

    it('should return "Local" for level 2', () => {
      expect(reachLevelName(2)).toBe('Local');
    });

    it('should return "Neighborhood" for level 3', () => {
      expect(reachLevelName(3)).toBe('Neighborhood');
    });

    it('should return "Municipal" for level 4', () => {
      expect(reachLevelName(4)).toBe('Municipal');
    });

    it('should return "Bioregional" for level 5', () => {
      expect(reachLevelName(5)).toBe('Bioregional');
    });

    it('should return "Regional" for level 6', () => {
      expect(reachLevelName(6)).toBe('Regional');
    });

    it('should return "Commons" for level 7', () => {
      expect(reachLevelName(7)).toBe('Commons');
    });

    it('should return fallback for invalid level', () => {
      expect(reachLevelName(8)).toBe('Level 8');
      expect(reachLevelName(99)).toBe('Level 99');
      expect(reachLevelName(-1)).toBe('Level -1');
    });
  });

  describe('statusColor()', () => {
    it('should return green for online status', () => {
      expect(statusColor('online')).toBe('text-green-600');
    });

    it('should return yellow for degraded status', () => {
      expect(statusColor('degraded')).toBe('text-yellow-600');
    });

    it('should return gray for offline status', () => {
      expect(statusColor('offline')).toBe('text-gray-500');
    });

    it('should return red for failed status', () => {
      expect(statusColor('failed')).toBe('text-red-600');
    });

    it('should return light blue for discovered status', () => {
      expect(statusColor('discovered')).toBe('text-blue-400');
    });

    it('should return blue for registering status', () => {
      expect(statusColor('registering')).toBe('text-blue-600');
    });

    it('should return gray for unknown status', () => {
      expect(statusColor('unknown-status' as NodeStatus)).toBe('text-gray-400');
    });
  });

  describe('tierColor()', () => {
    it('should return purple for pioneer tier', () => {
      expect(tierColor('pioneer')).toBe('text-purple-600');
    });

    it('should return indigo for steward tier', () => {
      expect(tierColor('steward')).toBe('text-indigo-600');
    });

    it('should return blue for guardian tier', () => {
      expect(tierColor('guardian')).toBe('text-blue-600');
    });

    it('should return green for caretaker tier', () => {
      expect(tierColor('caretaker')).toBe('text-green-600');
    });

    it('should return gray for null tier', () => {
      expect(tierColor(null)).toBe('text-gray-500');
    });

    it('should return gray for undefined tier', () => {
      expect(tierColor(undefined as unknown as StewardTier)).toBe('text-gray-500');
    });
  });
});
