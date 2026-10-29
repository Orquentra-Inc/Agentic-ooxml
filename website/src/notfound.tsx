import { Page } from './components/Chrome'
import { BASE } from './data'
import { mount } from './mount'

mount(
  <Page current="other">
    <section className="scene">
      <div className="wrap narrow">
        <p className="eyebrow"><span className="mono">404</span><span className="rule" aria-hidden="true" />Not found</p>
        <h1>This part is not in the package.</h1>
        <p className="lede">The page you asked for does not exist, or has moved.</p>
        <p><a className="btn" href={BASE}>Back to the start</a></p>
      </div>
    </section>
  </Page>,
)
