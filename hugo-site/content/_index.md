---
title: "Decentralize Everything"
date: 2024-06-11T00:00:00Z
draft: false
layout: "home"
---

Freenet is a peer-to-peer platform for decentralized applications—communication, collaboration, and
commerce without reliance on big tech. Your computer becomes part of a global network where apps
are unstoppable, interoperable, and built on open protocols.

<a href="/quickstart/" class="cta-button">Try Freenet</a>

<div class="action-and-news">

<div class="action-column">

## See it in Action

River is decentralized group chat built on Freenet. No servers to run or rely on, no admins who
control your data - just conversations that belong entirely to their participants.

<div class="screenshot-container">
<picture class="app-screenshot">
  <source srcset="/images/river-screenshot-dark.png" media="(prefers-color-scheme: dark)">
  <img src="/images/river-screenshot-light.png" alt="River - decentralized chat on Freenet">
</picture>
</div>

<script>
document.addEventListener('DOMContentLoaded', function() {
  var container = document.querySelector('.screenshot-container');
  if (!container) return;
  container.addEventListener('click', function() {
    var img = container.querySelector('img');
    var overlay = document.createElement('div');
    overlay.className = 'screenshot-lightbox';
    var clone = document.createElement('img');
    clone.src = img.currentSrc || img.src;
    clone.alt = img.alt;
    overlay.appendChild(clone);
    overlay.addEventListener('click', function() { overlay.remove(); });
    document.body.appendChild(overlay);
  });
});
</script>

</div>

<div class="news-column">

## News

{{< latest-news tag="front-page" include-releases="true" >}}

[More news...](/news/)

</div>

</div>

<div class="home-sections">

<div class="home-section">

### For Users

Freenet apps run in your browser and look like normal websites—but they can't be taken down,
don't track you, and work without any company behind them.

[Try it now →](/quickstart/)

</div>

<div class="home-section">

### For Developers

Build apps with familiar tools (Rust, TypeScript) that deploy to a global network. No servers to
maintain, no cloud bills, no terms of service.

[Read the Tutorial →](/resources/manual/tutorial/)

</div>

<div class="home-section">

### For Supporters

Freenet is built by a small team, funded through grants and donations. Your support helps build
decentralized internet infrastructure that matters.

<a href="/donate/" class="funding-learn-more">Support Freenet →</a>

</div>

</div>

<div class="secondary-links">

[Video Talks](/resources/video-talks/) ·
[Matrix Chat](https://matrix.to/#/#freenet-locutus:matrix.org) ·
[GitHub](https://github.com/freenet/freenet-core) ·
[FAQ](/faq/)

</div>
