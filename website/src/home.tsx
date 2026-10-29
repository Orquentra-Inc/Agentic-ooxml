import { Page } from './components/Chrome'
import { Hero } from './components/Hero'
import { StaleScene } from './components/StaleScene'
import { DiffScene } from './components/DiffScene'
import { RunScene } from './components/RunScene'
import { AudienceScene, CostScene, FaqScene, FinalCta, HighlightsScene, LimitsScene, QuestionsScene, WorksWithScene } from './components/Sell'
import { mount } from './mount'

mount(
  <Page current="home">
    <Hero />
    <CostScene />
    <QuestionsScene />
    <StaleScene />
    <DiffScene />
    <RunScene />
    <HighlightsScene />
    <AudienceScene />
    <WorksWithScene />
    <LimitsScene />
    <FaqScene />
    <FinalCta />
  </Page>,
)
