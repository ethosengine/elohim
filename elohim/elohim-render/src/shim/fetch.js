// `globalThis.fetch` backed by the Rust `op_fetch` op (a `DataFetcher`).
//
// Returns a real `Response` (from src/shim/web_api.js) with a `Headers` object
// and a `ReadableStream` body, because that is exactly the surface Angular's
// `FetchBackend` reads on the response path: `new HttpHeaders(response.headers)`
// only takes its fast path for a `Headers` instance, and the body is drained via
// `response.body.getReader()`. A plain `{status, headers: Map}` object -- what
// this shim returned before Angular 22 -- makes every successful SSR fetch
// deliver a null HTTP body.
//
// `AbortSignal` support is honest rather than cosmetic: `op_fetch` has no
// cancellation channel (the Rust `DataFetcher` owns its own soft-deadline, see
// crate::traced_fetcher), so an abort cannot stop work already in flight. What it
// CAN do is reject the caller's promise immediately with the signal's reason,
// which is what Angular's request-timeout leg observes. The in-flight op is left
// to settle and its result dropped.
//
// WARNING: pure ASCII (deno_core's `js = [dir ...]` macro panics on non-ASCII).
((globalThis) => {
  const streamOfBytes = globalThis[Symbol.for("elohim.render.streamOfBytes")];

  // Flatten any HeadersInit (Headers / array of pairs / plain object) into the
  // `HashMap<String, String>` shape `FetchRequest` deserializes.
  function headersToRecord(init) {
    const out = {};
    if (!init) {
      return out;
    }
    new Headers(init).forEach((value, name) => {
      out[name] = value;
    });
    return out;
  }

  // `FetchRequest.body` is `Option<Vec<u8>>`, so serialize to a byte array.
  function bodyToBytes(body) {
    if (body === undefined || body === null) {
      return null;
    }
    if (typeof body === "string") {
      return Array.from(new TextEncoder().encode(body));
    }
    if (body instanceof Uint8Array) {
      return Array.from(body);
    }
    if (ArrayBuffer.isView(body)) {
      return Array.from(
        new Uint8Array(body.buffer, body.byteOffset, body.byteLength)
      );
    }
    if (body instanceof ArrayBuffer) {
      return Array.from(new Uint8Array(body));
    }
    if (typeof Blob !== "undefined" && body instanceof Blob) {
      return Array.from(body._bytes);
    }
    // Anything else (FormData, URLSearchParams, ...) stringifies. Angular's
    // `serializeBody()` never hands us one of those for the SSR paths in play,
    // so this is a last resort rather than a supported shape.
    return Array.from(new TextEncoder().encode(String(body)));
  }

  globalThis.fetch = async (urlOrRequest, init) => {
    const opts = init || {};
    const isRequest =
      typeof Request !== "undefined" && urlOrRequest instanceof Request;

    const url = isRequest ? urlOrRequest.url : String(urlOrRequest);
    const method = String(
      opts.method || (isRequest ? urlOrRequest.method : "GET")
    ).toUpperCase();
    const headersInit =
      opts.headers !== undefined
        ? opts.headers
        : isRequest
          ? urlOrRequest.headers
          : null;
    const rawBody =
      opts.body !== undefined
        ? opts.body
        : isRequest
          ? urlOrRequest._bodyBytes
          : null;
    const signal =
      opts.signal !== undefined
        ? opts.signal
        : isRequest
          ? urlOrRequest.signal
          : null;

    // Fail before dispatching if the caller already gave up.
    if (signal && signal.aborted) {
      throw signal.reason !== undefined
        ? signal.reason
        : new DOMException("This operation was aborted", "AbortError");
    }

    const request = {
      method: method,
      url: url,
      headers: headersToRecord(headersInit),
      body: bodyToBytes(rawBody),
    };

    const opPromise = Deno.core.ops.op_fetch(request);

    let response;
    if (signal) {
      // Race the op against abort. The op keeps running on the Rust side; only
      // this promise's settlement is short-circuited (see the header note).
      let onAbort;
      const aborted = new Promise((_resolve, reject) => {
        onAbort = () =>
          reject(
            signal.reason !== undefined
              ? signal.reason
              : new DOMException("This operation was aborted", "AbortError")
          );
        signal.addEventListener("abort", onAbort, { once: true });
      });
      // A dropped race loser must not surface as an unhandled rejection.
      aborted.catch(() => {});
      try {
        response = await Promise.race([opPromise, aborted]);
      } finally {
        if (onAbort && typeof signal.removeEventListener === "function") {
          signal.removeEventListener("abort", onAbort);
        }
      }
    } else {
      response = await opPromise;
    }

    const bodyBytes = new Uint8Array(response.body);

    // Build the response Headers leniently: a real fetch() never throws
    // because an upstream server sent a header name/value the strict
    // Headers class (web_api.js's normalizeName/normalizeValue) would
    // reject -- skip-with-best-effort per entry instead of failing the
    // whole response. App-constructed `new Headers(...)` / `new Response(...,
    // {headers})` stays fully strict; only this wire-ingestion loop is
    // tolerant.
    const responseHeaders = new Headers();
    const rawHeaders = response.headers || {};
    for (const name of Object.keys(rawHeaders)) {
      try {
        responseHeaders.append(name, rawHeaders[name]);
      } catch (_ignored) {
        // Drop the one header the wire sent that strict validation rejects
        // rather than fail the whole response over it.
      }
    }

    return new Response(streamOfBytes(bodyBytes), {
      status: response.status,
      headers: responseHeaders,
      // The fetcher follows redirects opaquely and reports the requested URL,
      // so `redirected` is unknowable here -- absence over a fabricated value.
      url: url,
      type: "basic",
    });
  };
})(globalThis);
