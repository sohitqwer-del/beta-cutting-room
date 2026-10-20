/** Vite replaces import.meta.env.PROD at build time (true in `vite build`, false in the dev server and tests). */
interface ImportMetaEnv { readonly PROD: boolean; readonly VITE_BETA_STATIC?: string; readonly BASE_URL: string }
interface ImportMeta { readonly env: ImportMetaEnv }
