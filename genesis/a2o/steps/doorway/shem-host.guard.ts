/**
 * The `@requires:shem-host` gate -- E2E_SHEM_HOST (the URL of the shem peer) is a FIXTURE
 * precondition the lane supplies, not a substrate capability: `@requires:shem` can be declared
 * available in cluster-state while the runner (the genesis CI build) was never handed the host,
 * and the first Given then throws "E2E_SHEM_HOST not set". A scenario that needs the host is
 * SKIPPED with the reason printed once when it is unset; it still runs wherever the lane sets it.
 */
import { Before } from '@cucumber/cucumber';

let announced = false;

Before({ tags: '@requires:shem-host' }, function () {
  if ((process.env.E2E_SHEM_HOST ?? '').trim() !== '') return undefined;
  if (!announced) {
    announced = true;
    // eslint-disable-next-line no-console
    console.log(
      '  [shem-host] E2E_SHEM_HOST is not set -- @requires:shem-host scenarios SKIPPED, not measured'
    );
  }
  return 'skipped';
});
