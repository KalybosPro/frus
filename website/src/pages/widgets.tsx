import type {ReactNode} from 'react';
import Link from '@docusaurus/Link';
import Layout from '@theme/Layout';
import Heading from '@theme/Heading';

import styles from './widgets.module.css';

const gallery = 'https://kalybospro.github.io/frus/';
const source = 'https://github.com/KalybosPro/frus/tree/master/frus-examples/widgets/src/features';

// Mirrors the gallery's table of contents, `frus-examples/widgets/src/common/catalog.rs`.
const categories = [
  {id: 'layout', name: 'Layout', summary: 'Boxes, rows, columns, stacks and constraints', examples: ['Row & Column', 'Stack', 'Wrap', 'Align', 'SafeArea', 'LayoutBuilder']},
  {id: 'text', name: 'Text', summary: 'Text, styles, rich and selectable text', examples: ['Text', 'RichText', 'Selectable text', 'TextTheme']},
  {id: 'buttons', name: 'Buttons & menus', summary: 'Buttons, icon buttons, toggles and menus', examples: ['Button', 'IconButton', 'SegmentedButton', 'MenuAnchor']},
  {id: 'inputs', name: 'Inputs & forms', summary: 'Text fields, choices, sliders, pickers and validation', examples: ['TextField', 'Checkbox', 'Slider', 'Date picker', 'Autocomplete', 'Form']},
  {id: 'display', name: 'Display', summary: 'Cards, chips, avatars, icons, images, tables and charts', examples: ['Card', 'Chip', 'DataTable', 'Chart', 'Tree', 'Timeline']},
  {id: 'feedback', name: 'Feedback', summary: 'Dialogs, snack bars, sheets, banners and progress', examples: ['Dialog', 'SnackBar', 'Bottom sheet', 'Progress', 'Tooltip']},
  {id: 'navigation', name: 'Navigation', summary: 'App bars, tabs, navigation bars, rails and drawers', examples: ['AppBar', 'Tabs', 'NavigationBar', 'Drawer']},
  {id: 'scrolling', name: 'Lists & scrolling', summary: 'Lists, grids, pages, reordering and expansion', examples: ['ListView', 'GridView', 'PageView', 'Reorderable list']},
  {id: 'animation', name: 'Animation', summary: 'Implicit animations, transitions and heroes', examples: ['AnimatedContainer', 'AnimatedCrossFade', 'AnimatedSwitcher', 'Hero']},
  {id: 'painting', name: 'Painting & effects', summary: 'Decoration, clipping, transforms, custom painting and filters', examples: ['CustomPaint', 'Clip', 'Transform', 'BackdropFilter']},
  {id: 'interaction', name: 'Interaction', summary: 'Gestures, hover, drag and drop, focus and shortcuts', examples: ['GestureDetector', 'Drag and drop', 'MouseRegion', 'Shortcuts']},
  {id: 'state', name: 'State & async', summary: 'Hooks, notifiers, futures, streams, theme and screen size', examples: ['Hooks', 'StatefulWidget', 'FutureBuilder', 'MediaQuery']},
];

export default function Widgets(): ReactNode {
  return (
    <Layout title="Widget catalog" description="Every frus widget, grouped by family, each with a live example and the code that builds it.">
      <header className={styles.header}>
        <div className="container">
          <Heading as="h1">Widget catalog</Heading>
          <p className={styles.lead}>
            frus ships one widget library, <code>frus-widgets</code>, with themed defaults that every
            application can override. The gallery shows more than 170 demos, in twelve families.
            Each demo page has two tabs: <strong>Example</strong>, the widget running, and{' '}
            <strong>Code</strong>, the one file that builds it.
          </p>
          <div className={styles.buttons}>
            <Link className="button button--primary button--lg" href={gallery}>
              Open the live gallery
            </Link>
            <Link className="button button--secondary button--outline button--lg" href="https://docs.rs/frus-widgets">
              API reference
            </Link>
          </div>
          <p className={styles.note}>
            The gallery is frus itself, built for the web. It needs a browser with WebGPU, such as
            Chrome or Edge 113 or newer.
          </p>
        </div>
      </header>
      <main className="container">
        <div className={styles.grid}>
          {categories.map((c) => (
            <section key={c.id} className={styles.card}>
              <Heading as="h2">{c.name}</Heading>
              <p>{c.summary}</p>
              <ul className={styles.chips}>
                {c.examples.map((e) => (
                  <li key={e}>{e}</li>
                ))}
              </ul>
              <Link href={`${source}/${c.id}/widgets`}>Read the demos’ source →</Link>
            </section>
          ))}
        </div>
        <section className={styles.conventions}>
          <Heading as="h2">Conventions</Heading>
          <ul>
            <li>
              <strong>Every style can be overridden.</strong> A widget takes its colours, type and
              shape from the theme, never from a constant. You can restyle one instance, or every
              widget of a kind through the theme.
            </li>
            <li>
              <strong>Interactive widgets are controlled.</strong> A checkbox, a slider or a text
              field shows the value you give it, and tells you when the user changes it. The state
              stays yours.
            </li>
            <li>
              <strong>Accessible by default.</strong> Each widget tells screen readers what it is
              and what it shows. They can find it where it is drawn, and press it, tick it or fill
              it, on every platform.
            </li>
          </ul>
        </section>
      </main>
    </Layout>
  );
}
