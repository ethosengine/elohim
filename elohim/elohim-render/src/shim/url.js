// Minimal WHATWG URL implementation. deno_core's bare runtime (no snapshot)
// does not include URL as a global -- it must be injected via an extension.
// Sufficient for Angular SSR's use: absolute parsing, relative resolution, href,
// URLSearchParams, IPv6 authority, default-port elision.
//
// NOTE: URLSearchParams mutations (append/set/delete) do NOT back-propagate to
// the owning URL's _search field. Angular's HttpClient builds a new
// URLSearchParams, serializes via toString(), and assigns -- so this is safe.
// If Angular ever does url.searchParams.set(...) and expects url.search to
// update, a live-view wrapper is needed. TODO(task-9+): audit Angular HttpClient
// source to confirm the read-only access pattern holds.
((globalThis) => {
  // Default ports to elide from host/origin/href.
  const DEFAULT_PORTS = { "http:": "80", "https:": "443" };

  // Parse a fully-qualified URL string into parts.
  // Returns null if the string cannot be parsed as an absolute URL.
  function parseAbsolute(str) {
    // scheme://[userinfo@]host[:port][/path][?query][#fragment]
    const m = String(str).match(
      /^([a-zA-Z][a-zA-Z0-9+\-.]*):\/\/([^/?#]*)([^?#]*)(\?[^#]*)?(#.*)?$/
    );
    if (!m) return null;
    const [, scheme, authority, pathname, search, hash] = m;
    const protocol = scheme.toLowerCase() + ":";
    // Strip default port from authority so host/origin normalize correctly.
    let auth = authority;
    const defaultPort = DEFAULT_PORTS[protocol];
    if (defaultPort && auth.endsWith(":" + defaultPort)) {
      auth = auth.slice(0, -(defaultPort.length + 1));
    }
    return {
      protocol,
      authority: auth,
      pathname: pathname || "/",
      search: search || "",
      hash: hash || "",
    };
  }

  // Resolve a relative reference `rel` against an absolute base `base`.
  function resolve(rel, base) {
    rel = String(rel);
    // Already absolute?
    if (/^[a-zA-Z][a-zA-Z0-9+\-.]*:/.test(rel)) return rel;
    const b = parseAbsolute(base);
    if (!b) throw new TypeError("Invalid base URL: " + base);
    if (rel.startsWith("//")) {
      return b.protocol + rel;
    }
    if (rel.startsWith("/")) {
      return b.protocol + "//" + b.authority + rel;
    }
    if (rel.startsWith("?")) {
      return b.protocol + "//" + b.authority + b.pathname + rel;
    }
    if (rel.startsWith("#")) {
      return b.protocol + "//" + b.authority + b.pathname + b.search + rel;
    }
    // Relative path: merge with base path directory.
    const dir = b.pathname.replace(/\/[^/]*$/, "/");
    const merged = dir + rel;
    // Normalise dot segments.
    const segments = merged.split("/");
    const out = [];
    for (const seg of segments) {
      if (seg === "..") { out.pop(); }
      else if (seg !== ".") { out.push(seg); }
    }
    return b.protocol + "//" + b.authority + out.join("/");
  }

  // Bracket-aware hostname extraction from an authority string.
  // Handles IPv6 literals like [::1]:8080 without splitting on the wrong colon.
  function hostnameFromAuthority(auth) {
    if (auth.startsWith("[")) {
      const bracket = auth.indexOf("]");
      return bracket === -1 ? auth : auth.slice(0, bracket + 1);
    }
    const colon = auth.indexOf(":");
    return colon === -1 ? auth : auth.slice(0, colon);
  }

  // Bracket-aware port extraction from an authority string.
  function portFromAuthority(auth) {
    if (auth.startsWith("[")) {
      const bracket = auth.indexOf("]");
      if (bracket === -1 || bracket + 1 >= auth.length || auth[bracket + 1] !== ":") return "";
      return auth.slice(bracket + 2);
    }
    const colon = auth.indexOf(":");
    return colon === -1 ? "" : auth.slice(colon + 1);
  }

  class URLSearchParams {
    constructor(init) {
      this._params = [];
      if (init == null) return;
      if (typeof init === "string") {
        const s = init.startsWith("?") ? init.slice(1) : init;
        if (s) {
          for (const pair of s.split("&")) {
            const eq = pair.indexOf("=");
            const k = eq === -1 ? pair : pair.slice(0, eq);
            const v = eq === -1 ? "" : pair.slice(eq + 1);
            this._params.push([
              decodeURIComponent(k.replace(/\+/g, " ")),
              decodeURIComponent(v.replace(/\+/g, " ")),
            ]);
          }
        }
      } else if (Array.isArray(init)) {
        for (const [k, v] of init) this._params.push([String(k), String(v)]);
      } else if (typeof init === "object") {
        for (const k of Object.keys(init)) this._params.push([k, String(init[k])]);
      }
    }
    append(k, v) { this._params.push([String(k), String(v)]); }
    delete(k) { this._params = this._params.filter(p => p[0] !== k); }
    get(k) { const p = this._params.find(p => p[0] === k); return p ? p[1] : null; }
    getAll(k) { return this._params.filter(p => p[0] === k).map(p => p[1]); }
    has(k) { return this._params.some(p => p[0] === k); }
    set(k, v) {
      const idx = this._params.findIndex(p => p[0] === k);
      if (idx === -1) {
        this._params.push([String(k), String(v)]);
      } else {
        this._params[idx] = [String(k), String(v)];
        this._params = this._params.filter((p, i) => i === idx || p[0] !== k);
      }
    }
    *entries() { for (const p of this._params) yield [p[0], p[1]]; }
    *keys()    { for (const p of this._params) yield p[0]; }
    *values()  { for (const p of this._params) yield p[1]; }
    forEach(cb, thisArg) {
      for (const [k, v] of this._params) cb.call(thisArg, v, k, this);
    }
    toString() {
      return this._params
        .map(([k, v]) => encodeURIComponent(k) + "=" + encodeURIComponent(v))
        .join("&");
    }
    [Symbol.iterator]() { return this.entries(); }
  }
  globalThis.URLSearchParams = URLSearchParams;

  class URL {
    constructor(input, base) {
      input = String(input);
      let absolute;
      if (base !== undefined) {
        absolute = resolve(input, base);
      } else {
        absolute = input;
      }
      const parsed = parseAbsolute(absolute);
      if (!parsed) throw new TypeError("Invalid URL: " + input);
      this._protocol = parsed.protocol;
      this._authority = parsed.authority;
      this._pathname = parsed.pathname;
      this._search = parsed.search;
      this._hash = parsed.hash;
    }

    get href()     { return this._protocol + "//" + this._authority + this._pathname + this._search + this._hash; }
    get protocol() { return this._protocol; }
    get host()     { return this._authority; }
    get hostname() { return hostnameFromAuthority(this._authority); }
    get port()     { return portFromAuthority(this._authority); }
    get pathname() { return this._pathname; }
    get search()   { return this._search; }
    get hash()     { return this._hash; }
    get origin()   {
      const port = portFromAuthority(this._authority);
      return this._protocol + "//" + hostnameFromAuthority(this._authority) + (port ? ":" + port : "");
    }

    // Lazy: rebuild on each access since URLSearchParams mutations do not
    // back-propagate to _search. See module-level TODO for details.
    get searchParams() {
      return new URLSearchParams(this._search || "");
    }

    toString() { return this.href; }
    toJSON()   { return this.href; }

    static canParse(input, base) {
      try { new URL(input, base); return true; } catch { return false; }
    }
  }

  globalThis.URL = URL;
})(globalThis);
