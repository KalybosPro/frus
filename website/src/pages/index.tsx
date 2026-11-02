import type {ReactNode} from 'react';
import clsx from 'clsx';
import Link from '@docusaurus/Link';
import useBaseUrl from '@docusaurus/useBaseUrl';
import Layout from '@theme/Layout';
import CodeBlock from '@theme/CodeBlock';
import Heading from '@theme/Heading';

import styles from './index.module.css';

const repo = 'https://github.com/KalybosPro/frus';
const gallery = 'https://kalybospro.github.io/frus/';

function Hero(): ReactNode {
  return (
    <header className={styles.hero}>
      <div className={styles.heroGlow} aria-hidden />
      <div className="container">
        <img className={styles.heroLogo} src={useBaseUrl('/img/logo.png')} alt="" width={176} height={176} />
        <Heading as="h1" className={styles.heroTitle}>
          One Rust codebase.
          <br />
          <span className="frus-gradient-text">Every screen.</span>
        </Heading>
        <p className={styles.heroSubtitle}>
          frus is a UI framework written entirely in Rust. Build an application from components, and
          run it on Windows, Linux, macOS, Android and the web, drawn on the GPU by frus itself.
        </p>
        <div className={styles.heroButtons}>
          <Link className="button button--primary button--lg" to="/docs/getting-started">
            Get started
          </Link>
          <Link className="button button--secondary button--outline button--lg" href={gallery}>
            Widget gallery
          </Link>
        </div>
        <code className={styles.install}>cargo install frus-cli</code>
      </div>
    </header>
  );
}

const features: {title: string; body: ReactNode}[] = [
  {
    title: 'Components and hooks',
    body: (
      <>
        A widget is what it builds. Keep state with a <code>StatefulWidget</code>, or in a plain
        function with <code>use_state</code>, <code>use_memo</code> and <code>use_effect</code>.
      </>
    ),
  },
  {
    title: 'Rust from the app to the pixels',
    body: (
      <>
        Layout, text, widgets, gestures, theming and the renderer are all Rust. No VM, no second
        language, no bridge in the hot path.
      </>
    ),
  },
  {
    title: 'GPU rendering on wgpu',
    body: (
      <>
        Vulkan, Metal, DX12, OpenGL ES and WebGPU from one backend. Paths are tessellated with
        lyon, and text is shaped by cosmic-text.
      </>
    ),
  },
  {
    title: 'Desktop, Android and web',
    body: (
      <>
        One <code>frus::main!</code> wires every entry point. Screen readers, IME, insets and
        notifications work on each platform.
      </>
    ),
  },
  {
    title: 'Every widget restyles',
    body: (
      <>
        Light and dark themes are generated from a seed colour. Every widget takes its colours
        from the theme, so you can restyle one widget, or every widget of a kind.
      </>
    ),
  },
  {
    title: 'A command for the whole loop',
    body: (
      <>
        <code>frus create</code>, <code>run</code>, <code>build</code>, <code>test</code> and{' '}
        <code>doctor</code>, with hot reload that keeps the state. Whatever a command needs, it
        installs first.
      </>
    ),
  },
];

function Features(): ReactNode {
  return (
    <section className={styles.section}>
      <div className="container">
        <div className={styles.featureGrid}>
          {features.map((f) => (
            <div key={f.title} className={styles.card}>
              <Heading as="h3">{f.title}</Heading>
              <p>{f.body}</p>
            </div>
          ))}
        </div>
      </div>
    </section>
  );
}

const stats = [
  {value: '~2,180', label: 'tests in the repository, run on every change'},
  {value: '184', label: 'golden reference images, compared pixel by pixel'},
  {value: '170+', label: 'widget demos, each one shown with its code'},
  {value: '0.6–0.8 s', label: 'Windows cold start, down from 1.3–2.0 s in 0.4'},
];

function Numbers(): ReactNode {
  return (
    <section className={clsx(styles.section, styles.sectionAlt)}>
      <div className="container">
        <Heading as="h2" className={styles.sectionTitle}>
          Tested like it means it
        </Heading>
        <p className={styles.sectionLead}>
          Components run with no window and no GPU, so frus tests the whole lifecycle in plain
          Rust tests. Rendering is checked against reference images. Every example in the guide
          is compiled by the test suite.
        </p>
        <div className={styles.statGrid}>
          {stats.map((s) => (
            <div key={s.label} className={styles.stat}>
              <span className={clsx(styles.statValue, 'frus-gradient-text')}>{s.value}</span>
              <span className={styles.statLabel}>{s.label}</span>
            </div>
          ))}
        </div>
        <p className={styles.caveat}>
          These figures come from the repository and the changelog. frus does not publish
          side-by-side benchmarks against other toolkits yet: that comes once the numbers have
          been measured on real devices, with the method written down.
        </p>
      </div>
    </section>
  );
}

const counter = `use frus::{button, column, row, text, Align, BuildContext, FrusApp, Variant, Widget};

// A widget is what it builds. One that has something to remember keeps it with a hook.
fn counter(cx: &BuildContext) -> Box<dyn Widget> {
    let count = cx.use_state(|| 0);
    let (up, down) = (count.clone(), count.clone());
    Box::new(column![
        text(format!("{}", count.get())).size(48.0),
        row![
            button("+", move || up.update(|n| *n += 1)).variant(Variant::Filled),
            button("−", move || down.update(|n| *n -= 1)).variant(Variant::Outlined),
        ].gap(20.0),
    ].gap(16.0).align(Align::Center))
}

// One declaration wires up desktop, Android and web entry points.
frus::main!(FrusApp::from_fn(counter));`;

function Counter(): ReactNode {
  return (
    <section className={styles.section}>
      <div className={clsx('container', styles.split)}>
        <div className={styles.splitText}>
          <Heading as="h2" className={styles.sectionTitle}>
            A counter, in full
          </Heading>
          <p>
            That is a complete, runnable application. <code>cx.use_state</code> keeps the count
            between rebuilds, and the buttons update it. <code>frus::main!</code> declares the
            entry points for every platform at once.
          </p>
          <p>
            <code>cargo run</code> starts it on the desktop. <code>frus run -d android</code> and{' '}
            <code>frus run -d web</code> start it on a phone and in a browser. The source stays the same.
          </p>
          <Link to="/docs/getting-started">Write your first application →</Link>
        </div>
        <div className={styles.splitCode}>
          <CodeBlock language="rust" title="src/lib.rs">
            {counter}
          </CodeBlock>
        </div>
      </div>
    </section>
  );
}

const targets = [
  {
    title: 'Desktop',
    note: 'Windows, Linux and macOS: a window from winit, drawn with wgpu.',
    command: 'frus run',
  },
  {
    title: 'Android',
    note: 'A phone or an emulator. The first run installs the SDK, the NDK and Java.',
    command: 'frus run -d android',
  },
  {
    title: 'Web',
    note: 'Built to wasm, served, and opened in a WebGPU browser.',
    command: 'frus run -d web',
  },
  {
    title: 'Any phone, on any computer',
    note: 'A debug build shown in a phone’s frame, with its safe areas, cutout and keyboard.',
    command: 'frus run -d iphone-15',
  },
];

function Platforms(): ReactNode {
  return (
    <section className={clsx(styles.section, styles.sectionAlt)}>
      <div className="container">
        <Heading as="h2" className={styles.sectionTitle}>
          Build for every screen
        </Heading>
        <div className={styles.targetGrid}>
          {targets.map((t) => (
            <div key={t.title} className={styles.card}>
              <Heading as="h3">{t.title}</Heading>
              <p>{t.note}</p>
              <CodeBlock language="bash">{t.command}</CodeBlock>
            </div>
          ))}
        </div>
      </div>
    </section>
  );
}

const keys = [
  ['r', 'reload, keeping the state'],
  ['R', 'restart afresh'],
  ['i', 'the widget inspector'],
  ['P', 'a performance panel with graphs'],
  ['b', 'light or dark'],
  ['T', 'the text size'],
  ['o', 'another platform’s behaviour'],
  ['s', 'a screenshot'],
];

function HotReload(): ReactNode {
  return (
    <section className={styles.section}>
      <div className={clsx('container', styles.split)}>
        <div className={styles.splitText}>
          <Heading as="h2" className={styles.sectionTitle}>
            Change the code. Keep the state.
          </Heading>
          <p>
            <code>frus run --hot</code> patches each saved change into the running application, on
            the desktop, Android and the web. The log of every platform shows in your terminal,
            and single keys drive the application while it runs.
          </p>
          <ul className={styles.keys}>
            {keys.map(([k, what]) => (
              <li key={k}>
                <kbd>{k}</kbd> {what}
              </li>
            ))}
          </ul>
          <p>
            The VS Code extension adds a device picker, F5 with the log in the Debug Console, and
            reload on save.
          </p>
          <Link to="/docs/tooling/cli">The frus command →</Link>
        </div>
        <div className={styles.splitMedia}>
          <img
            src={useBaseUrl('/media/tour.gif')}
            alt="Four screens of the demo application, entered and left through frus's spring route transitions"
            loading="lazy"
          />
        </div>
      </div>
    </section>
  );
}

const shots = [
  {src: '/media/tasks.png', title: 'Widgets, gestures, theming', alt: 'A task list with an app bar, a segmented control, checkboxes and a floating action button'},
  {src: '/media/charts.png', title: 'Charts', alt: 'A line chart with a clickable legend above a grouped bar chart'},
  {src: '/media/board.png', title: 'Drag and drop', alt: 'A Kanban board of three columns of cards'},
  {src: '/media/data.png', title: 'Data tables', alt: 'A data table with search, sortable headers, row checkboxes and pagination'},
];

function Showcase(): ReactNode {
  const base = useBaseUrl('/');
  return (
    <section className={clsx(styles.section, styles.sectionAlt)}>
      <div className="container">
        <Heading as="h2" className={styles.sectionTitle}>
          What it looks like
        </Heading>
        <p className={styles.sectionLead}>
          These screens all come from one application. frus renders each picture through the same
          pipeline a window uses.
        </p>
        <div className={styles.shotGrid}>
          {shots.map((s) => (
            <figure key={s.src} className={styles.shot}>
              <img src={base + s.src.slice(1)} alt={s.alt} loading="lazy" />
              <figcaption>{s.title}</figcaption>
            </figure>
          ))}
        </div>
        <div className={styles.phoneRow}>
          <img src={base + 'media/android.png'} alt="The same application running on an Android phone" loading="lazy" />
          <div>
            <Heading as="h3">The same code on a phone</Heading>
            <p>
              Android is a first-class target, not a port: a native activity, Vulkan, a real IME
              with composition and swipe typing, system insets, and the lifecycle. This picture is
              a photograph of a device.
            </p>
            <Link href={gallery}>Open the widget gallery in your browser →</Link>
          </div>
        </div>
      </div>
    </section>
  );
}

const next = [
  {title: 'Get started', to: '/docs/getting-started', body: 'Install the command, make a project, run it on three platforms.'},
  {title: 'The guide', to: '/docs/guide', body: 'Layout, style, input, lists, navigation, network, animation, testing.'},
  {title: 'Widget catalog', to: '/widgets', body: 'Twelve families of widgets, each demo shown with the code that builds it.'},
  {title: 'Architecture', to: '/docs/internals/architecture', body: 'Four layers, and where each kind of code belongs.'},
];

function WhereNext(): ReactNode {
  return (
    <section className={styles.section}>
      <div className="container">
        <Heading as="h2" className={styles.sectionTitle}>
          Where to next
        </Heading>
        <div className={styles.nextGrid}>
          {next.map((n) => (
            <Link key={n.title} to={n.to} className={clsx(styles.card, styles.nextCard)}>
              <Heading as="h3">{n.title} →</Heading>
              <p>{n.body}</p>
            </Link>
          ))}
        </div>
      </div>
    </section>
  );
}

function Status(): ReactNode {
  return (
    <section className={clsx(styles.section, styles.status)}>
      <div className="container">
        <Heading as="h2" className={styles.sectionTitle}>
          frus is pre-alpha
        </Heading>
        <p className={styles.sectionLead}>
          The core is real, and three sample applications exercise it. The API is not stable,
          though, and any release may break it. Two gaps are known: there are no iOS or native
          macOS shells yet, and text rendering still has edge cases. At this stage, one pull
          request can shape a whole subsystem.
        </p>
        <div className={styles.heroButtons}>
          <Link className="button button--secondary button--outline" href={`${repo}/blob/master/ROADMAP.md`}>
            Read the roadmap
          </Link>
          <Link className="button button--secondary button--outline" href={`${repo}/issues`}>
            Open issues
          </Link>
        </div>
      </div>
    </section>
  );
}

export default function Home(): ReactNode {
  return (
    <Layout
      title="One Rust codebase. Every screen."
      description="frus is a cross-platform UI framework written entirely in Rust: desktop, Android and the web, GPU-rendered, built from components.">
      <Hero />
      <main>
        <Features />
        <Numbers />
        <Counter />
        <Platforms />
        <HotReload />
        <Showcase />
        <WhereNext />
        <Status />
      </main>
    </Layout>
  );
}
