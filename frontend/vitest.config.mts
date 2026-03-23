import { defineConfig } from 'vitest/config'
import react from '@vitejs/plugin-react'
import path from 'path'

export default defineConfig({
  plugins: [react()],
  resolve: {
    alias: {
      '@': path.resolve(__dirname, './src'),
    },
  },
  test: {
    environment: 'jsdom',
    globals: true,
    setupFiles: ['./src/test/setup.ts'],
    include: ['src/**/*.{test,spec}.{ts,tsx}'],
    exclude: ['node_modules', 'src/components/ui/**', '.next', 'src/test/e2e/**'],
    coverage: {
      provider: 'v8',
      reporter: ['text', 'json', 'html'],
      include: ['src/**/*.{ts,tsx}'],
      exclude: [
        'src/test/**',
        'src/**/*.test.{ts,tsx}',
        'src/**/*.spec.{ts,tsx}',
        // Next.js App Router pages are tested via component tests, not page tests
        'src/app/**',
        // Providers and layout boilerplate
        'src/components/providers/**',
        'src/**/*-provider.tsx',
        // Analytics chart components (recharts wrappers, no testable logic)
        'src/components/analytics/**',
        // Session components (require live Claude process)
        'src/components/sessions/**',
        // Layout header (simple presentational)
        'src/components/layout/header.tsx',
        // Settings panel (complex form, low ROI)
        'src/components/settings/**',
        // Skeleton/loading components (no logic)
        'src/**/*-skeleton.tsx',
        // Type-only files (no executable code)
        'src/types/**',
        // UI library shadcn components
        'src/components/ui/**',
        'src/app/globals.css',
      ],
      thresholds: {
        lines: 55,
        functions: 55,
        branches: 60,
        statements: 55,
      },
    },
  },
})
