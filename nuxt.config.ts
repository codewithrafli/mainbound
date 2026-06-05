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
      htmlAttrs: { class: 'dark' }
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
