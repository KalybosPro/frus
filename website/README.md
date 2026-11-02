# frus.dev — the frus website

The site: a home page, the documentation, the widget catalog and the blog, built with
[Docusaurus](https://docusaurus.io/).

```sh
npm install
npm start          # http://localhost:3000, reloads on change
npm run build      # the static site, in build/
```

## Where the pages come from

The documentation is written once, in the repository, where GitHub shows it and frus's test
suite compiles its examples. `npm start` and `npm run build` first run
`scripts/sync-docs.mjs`, which copies these files into `docs/` and points every link that
leaves them at GitHub:

| Repository | Site |
| --- | --- |
| `docs/getting-started.md` | `/docs/getting-started` |
| `docs/guide/*.md` | `/docs/guide/...` |
| `docs/app-structure.md` | `/docs/app-structure` |
| `crates/frus-cli/README.md` | `/docs/tooling/cli` |
| `ARCHITECTURE.md`, `CONTRIBUTING.md` | `/docs/internals/...` |
| `docs/media/*` (home page pictures) | `/media/...` |

**Edit the originals, not the copies:** `docs/` here is ignored by git, except `docs/intro.md`,
which belongs to the site. A new page needs an entry in the script and in `sidebars.ts`.

The rest is the site's own: `src/pages/index.tsx` (home), `src/pages/widgets.tsx` (catalog,
mirroring `frus-examples/widgets/src/common/catalog.rs`), `blog/`, and `src/css/custom.css`
(the palette, taken from the frus mark).

## Deployment

`SITE_URL` and `BASE_URL` set where the site is served (defaults: `https://frus.dev`, `/`):

```sh
SITE_URL=https://kalybospro.github.io BASE_URL=/frus/site/ npm run build
```
