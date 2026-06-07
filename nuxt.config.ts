// https://nuxt.com/docs/api/configuration/nuxt-config
export default defineNuxtConfig({
  modules: [
    '@nuxt/eslint',
    '@nuxt/ui',
    '@pinia/nuxt'
  ],

  // Tauri renders the SPA in a webview — no server
  ssr: false,

  devtools: {
    enabled: false
  },

  app: {
    head: {
      htmlAttrs: { class: 'dark' },
      script: [{
        // field-debugging safety net: the app background is pure black,
        // so any fatal startup error would otherwise look like a dead
        // black window. This inline script runs even if bundles fail.
        innerHTML: `(function(){
          function show(msg){
            var wrap = document.getElementById('mb-fatal-wrap');
            if (!wrap) {
              wrap = document.createElement('div');
              wrap.id = 'mb-fatal-wrap';
              wrap.style.cssText = 'position:fixed;left:12px;right:12px;bottom:12px;z-index:99999;';
              var btn = document.createElement('button');
              btn.textContent = '✕';
              btn.style.cssText = 'position:absolute;top:8px;right:10px;background:none;border:none;color:#fca5a5;font-size:14px;cursor:pointer;line-height:1;padding:2px 4px;';
              btn.onclick = function(){ wrap.remove(); };
              var el = document.createElement('pre');
              el.id = 'mb-fatal';
              el.style.cssText = 'max-height:45vh;overflow:auto;margin:0;padding:12px 32px 12px 14px;border-radius:10px;border:1px solid #7f1d1d;background:#1a0a0a;color:#fca5a5;font:11px ui-monospace,monospace;white-space:pre-wrap;';
              wrap.appendChild(el);
              wrap.appendChild(btn);
              document.documentElement.appendChild(wrap);
            }
            document.getElementById('mb-fatal').textContent += msg + '\\n';
          }
          window.addEventListener('error', function(e){
            show('[error] ' + (e.message || e.type) + (e.filename ? ' @ ' + e.filename + ':' + e.lineno : ''));
          }, true);
          window.addEventListener('unhandledrejection', function(e){
            show('[rejection] ' + (e.reason && (e.reason.stack || e.reason.message) || e.reason));
          });
          setTimeout(function(){
            var root = document.getElementById('__nuxt');
            if (!root || !root.firstChild) show('[fatal] UI failed to mount after 8s — app bundle did not start. macOS ' + navigator.userAgent);
          }, 8000);
        })();`
      }]
    }
  },

  css: ['~/assets/css/main.css'],

  ui: {
    colorMode: false
  },

  compatibilityDate: '2025-01-15',

  nitro: {
    preset: 'static'
  },

  vite: {
    clearScreen: false,
    envPrefix: ['VITE_', 'TAURI_'],
    optimizeDeps: {
      include: [
        '@git-diff-view/vue',
        '@tauri-apps/api/core',
        '@tauri-apps/api/event',
        '@tauri-apps/plugin-dialog',
        '@tauri-apps/plugin-notification',
        '@tauri-apps/plugin-opener',
        '@xterm/xterm',
        '@xterm/addon-fit',
        '@xterm/addon-search',
        '@xterm/addon-unicode11',
        '@xterm/addon-webgl'
      ]
    },
    server: {
      strictPort: true,
      watch: {
        ignored: ['**/src-tauri/**']
      }
    }
  },

  telemetry: false,

  eslint: {
    config: {
      stylistic: {
        commaDangle: 'never',
        braceStyle: '1tbs'
      }
    }
  }
})
