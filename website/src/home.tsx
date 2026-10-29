import { Page } from './components/Chrome'
import { Hero } from './components/Hero'
import { PackageScene } from './components/PackageScene'
import { LayersScene } from './components/LayersScene'
import { StaleScene } from './components/StaleScene'
import { WorkflowScene } from './components/WorkflowScene'
import { ArchScene } from './components/ArchScene'
import { InstallScene } from './components/InstallScene'
import { CloseScene } from './components/CloseScene'
import { mount } from './mount'

mount(
  <Page current="home">
    <Hero />
    <PackageScene />
    <LayersScene />
    <StaleScene />
    <WorkflowScene />
    <ArchScene />
    <InstallScene />
    <CloseScene />
  </Page>,
)
