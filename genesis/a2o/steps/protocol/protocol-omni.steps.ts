import { strict as assert } from 'node:assert';

import { Then } from '@cucumber/cucumber';

interface OmniWorld {
  doorwayResponse?: Response;
  doorwayBody?: Record<string, unknown>;
}

Then(
  'the response body has field {string} equal to {string}',
  async function (this: OmniWorld, field: string, expected: string) {
    if (!this.doorwayBody) {
      this.doorwayBody = (await this.doorwayResponse!.json()) as Record<string, unknown>;
    }
    assert.equal(this.doorwayBody[field], expected);
  },
);

Then(
  'the response body has field {string} which is an array',
  async function (this: OmniWorld, field: string) {
    if (!this.doorwayBody) {
      this.doorwayBody = (await this.doorwayResponse!.json()) as Record<string, unknown>;
    }
    assert.ok(
      Array.isArray(this.doorwayBody[field]),
      `${field} was ${typeof this.doorwayBody[field]} (${JSON.stringify(this.doorwayBody[field])})`,
    );
  },
);
