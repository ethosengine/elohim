// Minimal WHATWG URL implementation. deno_core's bare runtime (no snapshot)
// does not include URL as a global -- it must be injected via an extension.
// Sufficient for Angular SSR's use: absolute parsing, relative resolution, href.
((globalThis) => {
  // Parse a fully-qualified URL string into parts.
  // Returns null if the string cannot be parsed as an absolute URL.
  function parseAbsolute(str) {
    // scheme://[userinfo@]host[:port][/path][?query][#fragment]
    const m = String(str).match(
      /^([a-zA-Z][a-zA-Z0-9+\-.]*):\/\/([^/?#]*)([^?#]*)(\?[^#]*)?(#.*)?$/
    );
    if (!m) return null;
    const [, scheme, authority, pathname, search, hash] = m;
    return {
      protocol: scheme.toLowerCase() + ":",
      authority,
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
    get hostname() { return this._authority.split(":")[0]; }
    get port()     { return this._authority.includes(":") ? this._authority.split(":")[1] : ""; }
    get pathname() { return this._pathname; }
    get search()   { return this._search; }
    get hash()     { return this._hash; }
    get origin()   { return this._protocol + "//" + this.hostname + (this.port ? ":" + this.port : ""); }

    toString() { return this.href; }
    toJSON()   { return this.href; }

    static canParse(input, base) {
      try { new URL(input, base); return true; } catch { return false; }
    }
  }

  globalThis.URL = URL;
})(globalThis);
