// Intentionally select the local stations within the broader @wip feature.
// Household hooks, UI fixtures and remote services are outside this local proof.
export default () => ({
  default: {
    requireModule: ['tsx'],
    require: ['steps/devflow/shared-working-versions.steps.ts'],
    paths: ['features/devflow/shared-working-versions.feature'],
    tags: '@local-native',
    format: ['progress'],
    parallel: 0,
  },
});
