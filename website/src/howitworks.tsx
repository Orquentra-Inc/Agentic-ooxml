import { Page } from './components/Chrome'
import { PackageScene } from './components/PackageScene'
import { LayersScene } from './components/LayersScene'
import { WorkflowScene } from './components/WorkflowScene'
import { ArchScene } from './components/ArchScene'
import { InstallScene } from './components/InstallScene'
import { BASE, docsUrl } from './data'
import { mount } from './mount'

mount(
  <Page current="how">
    <section className="scene page-top" aria-labelledby="h-how">
      <div className="wrap narrow">
        <p className="eyebrow"><span className="mono">How it works</span><span className="rule" aria-hidden="true" />The technical tour</p>
        <h1 id="h-how">Under the hood: parts, layers, tools.</h1>
        <p className="lede">
          The home page is about outcomes. This page is the mechanism: how an Office file is a package of XML parts, what Agentic OOXML adds
          to it, how an agent is meant to use it, and every way to install it. Interactive pieces here are simulations over synthetic data.
        </p>
        <p className="cta-row">
          <a className="btn btn-quiet" href={BASE}>Back to the overview</a>
          <a className="btn btn-quiet" href={docsUrl('docs/schema.md')}>Schema reference</a>
        </p>
      </div>
    </section>
    <PackageScene />
    <LayersScene />
    <WorkflowScene />
    <ArchScene />
    <InstallScene />
  </Page>,
)
