---
title: "Freeing the Internet"
date: 2026-07-17
description: "A Freenet progress update: group chat, publishing, search, mail, and the other core internet services now running on Freenet, plus a no-install way to try them at try.freenet.org."
author: "Ian Clarke"
layout: single
type: presentations
customCss: |
  .reveal { font-size: 24px; }
  .reveal h1 { font-size: 2.2em; }
  .reveal h2 { font-size: 1.12em; margin-bottom: 0.25em; line-height: 1.15; }
  /* reveal.js sets inline display:block on the shown slide, killing the flex
     column so content collapses to the top; force it back on the visible slide */
  .reveal .slides section.present { display: flex !important; flex-direction: column; justify-content: safe center; }
  .reveal section { padding: 0.4em 1.8em !important; }
  .reveal li { margin-bottom: 0.4em; }
  .reveal blockquote { padding: 0.8em 1.2em; }
slides:
  # 1. Title
  - futo/title
  # 2. Why now (the censorship machinery being mandated)
  - futo/why-now
  # 3. What Freenet is (grounding for newcomers)
  - futo/services-without-servers
  # 4. What changed
  - futo/since-last-time
  # 3. The on-ramp
  - futo/try-in-browser
  # 4. The platform
  - futo/ecosystem-map
  - futo/app-status
  # 5. River demo + private rooms
  - futo/demo-river
  - futo/river-no-backend
  - futo/private-rooms
  - futo/private-rooms-limits
  # 6. The Signal question
  - futo/signal-contrast
  # 7. The shared pattern
  - futo/same-pattern
  # 8. Beyond River
  - futo/delta
  # Atlas — reworked as the multi-slide per-app section
  - futo/atlas/discovery
  - futo/atlas/today
  - futo/atlas/the-line
  - futo/atlas/decentralizes
  - futo/freenet-git
  - futo/whats-next
  # 9. Under the hood
  - futo/contracts
  - futo/renegade
  - futo/demand-hosting
  - futo/issue-prioritizer
  # 10. Close
  - futo/closing
  - common/questions
---
