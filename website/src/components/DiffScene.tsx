import { SectionHead } from './Chrome'
import { Reveal } from './Reveal'
import { DiffAnim } from './DiffAnim'

export function DiffScene() {
  return (
    <section id="review" className="scene" aria-labelledby="h-review">
      <div className="wrap">
        <SectionHead n="05" total="12" eyebrow="Proof, continued" id="h-review" lede="diff compares two copies of a file and reports which native parts changed, by hash, and which source statuses moved. It shows you what an agent did, in a form you can read before you accept it.">
          Every edit, reviewable.
        </SectionHead>
        <Reveal><DiffAnim /></Reveal>
      </div>
    </section>
  )
}
