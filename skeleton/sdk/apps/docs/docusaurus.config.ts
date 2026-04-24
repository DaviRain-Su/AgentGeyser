import type { Config } from '@docusaurus/types';
import type * as Preset from '@docusaurus/preset-classic';

const config: Config = {
  title: 'AgentGeyser SDK',
  tagline: 'Embedding AI agents natively into Solana RPC',
  favicon: 'img/favicon.ico',
  url: 'https://docs.agentgeyser.dev',
  baseUrl: '/',
  onBrokenLinks: 'throw',
  onBrokenMarkdownLinks: 'throw',
  trailingSlash: false,

  i18n: {
    defaultLocale: 'en',
    locales: ['en'],
  },

  presets: [
    [
      '@docusaurus/preset-classic',
      {
        docs: {
          sidebarPath: './sidebars.ts',
        },
        blog: false,
        theme: {
          customCss: './src/css/custom.css',
        },
      } satisfies Preset.Options,
    ],
  ],

  themeConfig: {
    navbar: {
      title: 'AgentGeyser SDK',
      items: [
        { type: 'docSidebar', sidebarId: 'docsSidebar', position: 'left', label: 'Docs' },
      ],
    },
    // TODO: replace with real Algolia appId/apiKey once DocSearch is approved
    algolia: {
      appId: 'PLACEHOLDER',
      apiKey: 'PLACEHOLDER',
      indexName: 'agentgeyser',
      contextualSearch: true,
    },
  } satisfies Preset.ThemeConfig,
};

export default config;
