// @ts-check
import { defineConfig } from 'astro/config';

// Static output. Every page is derivable from the JSON feed at build time, so there is
// nothing for a server to do at request time — and a static site can be served from the
// same place the repository lives.
export default defineConfig({
  output: 'static',
  trailingSlash: 'ignore',
  build: {
    // Charts are rendered to SVG during the build, so pages ship no client JavaScript.
    inlineStylesheets: 'auto',
  },
  markdown: {
    syntaxHighlight: false,
  },
});
