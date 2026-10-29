import { motion, useReducedMotion } from 'motion/react'
import type { ReactNode } from 'react'

/** Fades and lifts content once as it enters view. Static when the user prefers reduced motion. */
export function Reveal({ children, delay = 0, className, as = 'div' }: { children: ReactNode; delay?: number; className?: string; as?: 'div' | 'li' | 'p' }) {
  const reduce = useReducedMotion()
  const M = motion[as]
  if (reduce) return <M className={className}>{children}</M>
  return (
    <M
      className={className}
      initial={{ opacity: 0, y: 14 }}
      whileInView={{ opacity: 1, y: 0 }}
      viewport={{ once: true, margin: '0px 0px -8% 0px' }}
      transition={{ duration: 0.6, delay, ease: [0.2, 0.7, 0.2, 1] }}
    >
      {children}
    </M>
  )
}
