import resolve from '@rollup/plugin-node-resolve';
import commonjs from '@rollup/plugin-commonjs';
import replace from '@rollup/plugin-replace';
import url from '@rollup/plugin-url';
import esbuild from 'rollup-plugin-esbuild';
import alias from '@rollup/plugin-alias';
import path from 'path';
import { fileURLToPath } from 'url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));

// Process shim for browser environment
// Injected at the top of the bundle to prevent "process is not defined" errors
const processShim = `
(function() {
  // Process shim
  if (typeof globalThis !== 'undefined' && typeof globalThis.process === 'undefined') {
    globalThis.process = {
      env: { NODE_ENV: 'production' },
      cwd: function() { return '/'; },
      platform: 'browser',
      version: '',
      nextTick: function(fn) { setTimeout(fn, 0); }
    };
  }

  // Global polyfill - ensure Array.from is available (needed by some React internals)
  if (typeof Array.from !== 'function') {
    Array.from = function(arrayLike, mapFn, thisArg) {
      var arr = [];
      var len = arrayLike.length >>> 0;
      for (var i = 0; i < len; i++) {
        if (i in arrayLike) {
          arr.push(mapFn ? mapFn.call(thisArg, arrayLike[i], i) : arrayLike[i]);
        }
      }
      return arr;
    };
  }

  // Ensure Set constructor accepts an iterable (used by React DOM event system)
  // Some environments have incomplete Set implementations
  var _OriginalSet = typeof Set !== 'undefined' ? Set : undefined;
  if (_OriginalSet) {
    try {
      // Test if Set properly handles concat'd arrays in constructor
      new _OriginalSet([].concat(['test']));
    } catch (e) {
      // Set constructor doesn't work as expected, polyfill won't help here
      console.warn('[Perseus] Set polyfill test failed, may have initialization issues');
    }
  }

  // jQuery/vmouse shim - graphie code uses vmouse for touch handling
  // If jQuery isn't available, stub out the vmouse events
  if (typeof window !== 'undefined' && typeof window.jQuery === 'undefined') {
    console.log('[Perseus] No jQuery detected, stubbing vmouse...');
    // Create minimal jQuery stub to prevent vmouse initialization errors
    window.jQuery = window.$ = function(selector) {
      return {
        on: function() { return this; },
        off: function() { return this; },
        trigger: function() { return this; },
        bind: function() { return this; },
        unbind: function() { return this; },
        each: function() { return this; },
        length: 0
      };
    };
    window.jQuery.fn = window.jQuery.prototype = {};
    window.jQuery.extend = function(target) {
      for (var i = 1; i < arguments.length; i++) {
        var source = arguments[i];
        for (var key in source) {
          if (source.hasOwnProperty(key)) {
            target[key] = source[key];
          }
        }
      }
      return target;
    };
    window.jQuery.event = {
      special: {},
      add: function() {},
      remove: function() {},
      // Props array used by vmouse for mouseEventProps = $.event.props.concat(mouseHookProps)
      props: ['altKey', 'bubbles', 'cancelable', 'ctrlKey', 'currentTarget',
              'eventPhase', 'metaKey', 'relatedTarget', 'shiftKey', 'target',
              'timeStamp', 'view', 'which'],
      mouseHooks: {
        props: ['button', 'buttons', 'clientX', 'clientY', 'offsetX', 'offsetY',
                'pageX', 'pageY', 'screenX', 'screenY', 'toElement']
      }
    };
    // vmouse configuration
    window.jQuery.vmouse = {
      moveDistanceThreshold: 10,
      clickDistanceThreshold: 10,
      resetTimerDuration: 1500
    };
    // jQuery data storage
    window.jQuery.data = function() { return {}; };
    window.jQuery.Event = function(event) { return event; };
    console.log('[Perseus] jQuery stub installed');
  }
})();
`;

// Wrapper to catch initialization errors gracefully
// We wrap the entire bundle content in a try-catch because some dependencies
// have non-fatal initialization errors that would otherwise stop execution
const errorBoundaryWrapper = `
console.log('[Perseus] Bundle initialization starting...');
try {
`;

// Footer to close the try-catch AND ensure registration happens
const errorBoundaryFooter = `
} catch (e) {
  console.warn('[Perseus] Non-fatal initialization error caught:', e.message);
  console.warn('[Perseus] Stack:', e.stack);
}

// Ensure custom element registration happens even after caught errors
// This runs OUTSIDE the try-catch, so initialization errors don't block it
(function() {
  console.log('[Perseus] Post-init registration check...');
  if (typeof window !== 'undefined' && typeof customElements !== 'undefined') {
    // Give the bundle a moment to settle, then register
    setTimeout(function() {
      if (!customElements.get('perseus-question')) {
        console.log('[Perseus] Attempting fallback registration...');
        // Access the PerseusPlugin global (UMD name)
        if (typeof PerseusPlugin !== 'undefined' && PerseusPlugin.registerPerseusElement) {
          try {
            PerseusPlugin.registerPerseusElement();
            console.log('[Perseus] Fallback registration succeeded');
          } catch (err) {
            console.error('[Perseus] Fallback registration failed:', err);
          }
        } else {
          console.warn('[Perseus] PerseusPlugin not available for fallback registration');
        }
      } else {
        console.log('[Perseus] Custom element already registered');
      }
    }, 0);
  }
})();
`;

// Plugin to patch bundled jQuery's event.props (removed in jQuery 3.x but needed by vmouse)
const patchJQueryEventProps = () => ({
  name: 'patch-jquery-event-props',
  renderChunk(code) {
    // Find the jQuery definition and inject the props patch right after it
    const jqueryDefPattern = /var \$ = \/\*@__PURE__\*\/getDefaultExportFromCjs\$1\(jqueryExports\);/;
    const match = code.match(jqueryDefPattern);
    if (match) {
      const patch = `
// Patch jQuery event.props for vmouse compatibility (removed in jQuery 3.x)
if ($ && $.event && !$.event.props) {
  $.event.props = ['altKey', 'bubbles', 'cancelable', 'ctrlKey', 'currentTarget',
    'eventPhase', 'metaKey', 'relatedTarget', 'shiftKey', 'target', 'timeStamp', 'view', 'which'];
  $.event.mouseHooks = $.event.mouseHooks || {
    props: ['button', 'buttons', 'clientX', 'clientY', 'offsetX', 'offsetY',
            'pageX', 'pageY', 'screenX', 'screenY', 'toElement']
  };
  console.log('[Perseus] Patched bundled jQuery event.props');
}
`;
      return code.replace(jqueryDefPattern, match[0] + patch);
    }
    return code;
  }
});

export default {
  input: 'src/index.ts',
  output: [
    {
      file: 'dist/perseus-plugin.umd.js',
      format: 'umd',
      name: 'PerseusPlugin',
      sourcemap: true,
      inlineDynamicImports: true,
      intro: errorBoundaryWrapper + processShim,
      outro: errorBoundaryFooter,
      globals: {
        'react': 'React',
        'react-dom': 'ReactDOM',
        'react-dom/client': 'ReactDOM',
        'react/jsx-runtime': 'React'
      }
    },
    {
      file: 'dist/index.esm.js',
      format: 'es',
      sourcemap: true,
      inlineDynamicImports: true,
      intro: errorBoundaryWrapper + processShim
    }
  ],
  // Mark only React core as external - jsx-runtime needs to be bundled
  external: ['react', 'react-dom', 'react-dom/client'],
  plugins: [
    // Fix asap package browser resolution - the browser field mappings
    // in asap/package.json aren't being applied correctly by node-resolve
    alias({
      entries: [
        {
          find: /^asap\/raw$/,
          replacement: path.resolve(__dirname, 'node_modules/asap/browser-raw.js')
        },
        {
          find: /^\.\/raw$/,
          replacement: path.resolve(__dirname, 'node_modules/asap/browser-raw.js'),
          customResolver: (_source, importer) => {
            // Only apply this alias when the importer is from the asap package
            if (importer && importer.includes('asap')) {
              return path.resolve(__dirname, 'node_modules/asap/browser-raw.js');
            }
            return null;
          }
        }
      ]
    }),
    // Handle SVG imports from @phosphor-icons/core
    url({
      include: ['**/*.svg', '**/*.png', '**/*.jpg', '**/*.gif'],
      limit: 0, // Always inline as base64
      emitFiles: false
    }),
    // TypeScript/TSX compilation via esbuild (fast, handles JSX natively)
    esbuild({
      include: /\.[jt]sx?$/,
      exclude: /node_modules/,
      target: 'es2020',
      jsx: 'automatic',
      jsxImportSource: 'react',
      tsconfig: './tsconfig.json'
    }),
    // Replace process.env.NODE_ENV for production optimization
    // This runs after the intro shim, so any remaining references work
    replace({
      'process.env.NODE_ENV': JSON.stringify('production'),
      preventAssignment: true
    }),
    resolve({
      browser: true,
      preferBuiltins: false,
      extensions: ['.js', '.jsx', '.ts', '.tsx']
    }),
    commonjs({
      include: /node_modules/,
      transformMixedEsModules: true
    }),
    // Patch bundled jQuery event.props for vmouse compatibility
    patchJQueryEventProps()
  ],
  onwarn(warning, warn) {
    // Suppress circular dependency warnings from React ecosystem
    if (warning.code === 'CIRCULAR_DEPENDENCY') return;
    // Suppress "use client" directive warnings
    if (warning.code === 'MODULE_LEVEL_DIRECTIVE') return;
    warn(warning);
  }
};
