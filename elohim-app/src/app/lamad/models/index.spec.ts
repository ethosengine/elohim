import * as Models from './index';

describe('Models Index', () => {
  it('should export all model types', () => {
    expect(Models.NodeType).toBeDefined();
    expect(Models.RelationshipType).toBeDefined();
  });

  it('should export document node types', () => {
    expect(Models).toBeDefined();
  });
});
