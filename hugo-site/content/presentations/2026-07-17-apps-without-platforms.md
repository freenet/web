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
  - futo/stack-comparison
  # 4. What changed since the last talk
  - futo/since-last-time-sphere
  # 5. The on-ramp
  - futo/try-in-browser
  # 6. How Freenet works (platform, before the apps)
  - futo/contracts
  - futo/app-delivery
  - futo/under-the-hood
  - futo/renegade
  # 7. The apps
  - futo/app-status
  - futo/river-no-backend
  - futo/private-rooms
  - futo/private-rooms-limits
  - futo/delta
  - futo/atlas/discovery
  - futo/freenet-git
  - futo/whats-next
  - futo/composable
  # 8. Close
  - futo/closing
  - futo/questions
  # CUT for time (files kept, restore by re-adding):
  #   futo/ecosystem-map        (overlapped app-status)
  #   futo/atlas/today          (Atlas 4 slides -> 2)
  #   futo/atlas/decentralizes  (Atlas 4 slides -> 2)
  #   futo/demand-hosting       (partly-done redesign, needs hedging)
  #   futo/issue-prioritizer    (about running the project, not the thesis)
  #   futo/signal-contrast      (folded into private-rooms-limits; value-prop redundant with slides 3 and 8)
---
