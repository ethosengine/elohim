((globalThis) => {
  globalThis.fetch = async (urlOrRequest, init) => {
    const url = typeof urlOrRequest === "string" ? urlOrRequest : urlOrRequest.url;
    const opts = init || {};
    const request = {
      method: opts.method || "GET",
      url: url,
      headers: opts.headers || {},
      body: opts.body
        ? Array.from(typeof opts.body === "string"
            ? new TextEncoder().encode(opts.body)
            : opts.body)
        : null,
    };
    const response = await Deno.core.ops.op_fetch(request);
    const bodyBytes = new Uint8Array(response.body);
    return {
      status: response.status,
      headers: new Map(Object.entries(response.headers)),
      arrayBuffer: async () => bodyBytes.buffer,
      text: async () => new TextDecoder().decode(bodyBytes),
      json: async () => JSON.parse(new TextDecoder().decode(bodyBytes)),
      ok: response.status >= 200 && response.status < 300,
    };
  };
})(globalThis);
