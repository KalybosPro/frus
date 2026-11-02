import type {SidebarsConfig} from '@docusaurus/plugin-content-docs';

// Every page but `intro` is copied from the repository by `scripts/sync-docs.mjs`.

const sidebars: SidebarsConfig = {
  docs: [
    'intro',
    'getting-started',
    {
      type: 'category',
      label: 'The guide',
      link: {type: 'doc', id: 'guide/README'},
      collapsed: false,
      items: [
        'guide/layout',
        'guide/style',
        'guide/input',
        'guide/lists',
        'guide/navigation',
        'guide/network',
        'guide/animation',
        'guide/testing',
        'guide/platform',
      ],
    },
    'app-structure',
    {
      type: 'category',
      label: 'Tooling',
      items: ['tooling/cli'],
    },
    {
      type: 'category',
      label: 'Internals',
      items: ['internals/architecture', 'internals/contributing'],
    },
    {
      type: 'link',
      label: 'API reference',
      href: 'https://docs.rs/frus',
    },
  ],
};

export default sidebars;
