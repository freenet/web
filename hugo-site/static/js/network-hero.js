/**
 * Freenet Network Hero Animation
 * Standalone ring topology visualization with animated message particles.
 * Designed for the freenet.org homepage — no external dependencies.
 */
(function () {
  'use strict';

  // --- Configuration ---
  const SIZE = 450;
  const CENTER = SIZE / 2;
  const RADIUS = 185;
  const PEER_COUNT = 112;
  const PARTICLE_DURATION = 900;   // ms per particle travel
  const LOOP_DURATION = 10000;     // ms for full replay cycle
  const SPAWN_INTERVAL = 120;      // ms between particle spawns
  const MAX_PARTICLES = 60;

  // Event type colors (matches dashboard)
  const COLORS = {
    connect:   '#7ecfef',
    put:       '#fbbf24',
    get:       '#34d399',
    update:    '#a78bfa',
    subscribe: '#f472b6',
  };
  const COLOR_LIST = Object.values(COLORS);
  const TYPE_NAMES = Object.keys(COLORS);

  // Particle shape styles per event type
  const STYLES = {
    connect:   'circle',
    put:       'circle',
    get:       'diamond',
    update:    'triangle',
    subscribe: 'circle',
  };

  // --- Theme detection ---
  function isDark() {
    return window.matchMedia('(prefers-color-scheme: dark)').matches;
  }

  function ringColor()   { return isDark() ? '#1a2a2a' : '#d0d8e0'; }
  function glowColor()   { return isDark() ? 'rgba(0, 212, 170, 0.08)' : 'rgba(0, 140, 120, 0.06)'; }
  function peerColor()   { return isDark() ? '#007FFF' : '#3388dd'; }
  function peerGlow()    { return isDark() ? 'rgba(0, 127, 255, 0.18)' : 'rgba(0, 100, 200, 0.12)'; }
  function gwColor()     { return '#f59e0b'; }
  function labelColor()  { return isDark() ? '#484f58' : '#8090a0'; }
  function innerRing()   { return isDark() ? 'rgba(255,255,255,0.03)' : 'rgba(0,0,0,0.04)'; }
  function bgColor()     { return isDark() ? '#0d1117' : '#ffffff'; }

  // --- Geometry helpers ---
  function locationToXY(loc) {
    var a = loc * 2 * Math.PI - Math.PI / 2;
    return { x: CENTER + RADIUS * Math.cos(a), y: CENTER + RADIUS * Math.sin(a) };
  }

  function controlPoint(fromLoc, toLoc) {
    var a1 = fromLoc * 2 * Math.PI - Math.PI / 2;
    var a2 = toLoc * 2 * Math.PI - Math.PI / 2;
    var diff = a2 - a1;
    while (diff > Math.PI) diff -= 2 * Math.PI;
    while (diff < -Math.PI) diff += 2 * Math.PI;
    var mid = a1 + diff / 2;
    var pull = 0.3 + (Math.abs(diff) / Math.PI) * 0.5;
    var r = RADIUS * (1 - pull);
    return { x: CENTER + r * Math.cos(mid), y: CENTER + r * Math.sin(mid) };
  }

  function quadBezier(from, cp, to, t) {
    var u = 1 - t;
    return {
      x: u * u * from.x + 2 * u * t * cp.x + t * t * to.x,
      y: u * u * from.y + 2 * u * t * cp.y + t * t * to.y,
    };
  }

  // --- Deterministic pseudo-random (seeded for reproducibility) ---
  var _seed = 42;
  function srand(s) { _seed = s; }
  function rand() {
    _seed = (_seed * 1103515245 + 12345) & 0x7fffffff;
    return _seed / 0x7fffffff;
  }

  // --- Generate synthetic peers ---
  function generatePeers(count) {
    srand(7919); // prime seed for nice distribution
    var peers = [];
    for (var i = 0; i < count; i++) {
      var loc = rand();
      peers.push({
        location: loc,
        pos: locationToXY(loc),
        isGateway: i === 0,
      });
    }
    // Sort by location for visual consistency
    peers.sort(function (a, b) { return a.location - b.location; });
    // Ensure first peer is gateway (closest to 0.0)
    peers[0].isGateway = true;
    return peers;
  }

  // --- Generate synthetic connections ---
  function generateConnections(peers) {
    srand(1337);
    var conns = [];
    var n = peers.length;
    // Each peer connects to 2-4 nearest neighbors on the ring
    for (var i = 0; i < n; i++) {
      var numConns = 2 + Math.floor(rand() * 3);
      for (var j = 0; j < numConns; j++) {
        var offset = 1 + Math.floor(rand() * Math.min(6, Math.floor(n / 3)));
        var target = (i + offset) % n;
        if (target !== i) {
          conns.push([i, target]);
        }
      }
    }
    return conns;
  }

  // --- Generate synthetic message flows ---
  function generateFlows(peers, connections) {
    srand(2025);
    var flows = [];
    var count = 120; // enough to fill 10s loop densely

    for (var i = 0; i < count; i++) {
      // Pick a random connection or random peer pair
      var fromIdx, toIdx;
      if (rand() < 0.7 && connections.length > 0) {
        var conn = connections[Math.floor(rand() * connections.length)];
        fromIdx = conn[0];
        toIdx = conn[1];
        if (rand() < 0.5) { var tmp = fromIdx; fromIdx = toIdx; toIdx = tmp; }
      } else {
        fromIdx = Math.floor(rand() * peers.length);
        toIdx = Math.floor(rand() * peers.length);
        if (toIdx === fromIdx) toIdx = (toIdx + 1) % peers.length;
      }

      // Weight distribution to reflect real network traffic:
      // update_broadcast (~45%), connect (~27%), subscribe (~18%), get (~8%), put (~2%)
      var r2 = rand();
      var typeIdx;
      if (r2 < 0.40) typeIdx = 3;      // update (purple triangles - most common)
      else if (r2 < 0.62) typeIdx = 0;  // connect (cyan circles)
      else if (r2 < 0.80) typeIdx = 4;  // subscribe (pink circles)
      else if (r2 < 0.95) typeIdx = 2;  // get (green diamonds)
      else typeIdx = 1;                  // put (amber circles - rarest)

      var typeName = TYPE_NAMES[typeIdx];

      flows.push({
        from: fromIdx,
        to: toIdx,
        color: COLORS[typeName],
        style: STYLES[typeName],
        offsetMs: (i / count) * LOOP_DURATION,
      });
    }

    flows.sort(function (a, b) { return a.offsetMs - b.offsetMs; });
    return flows;
  }

  // --- Draw static ring elements ---
  function drawRing(ctx) {
    var dark = isDark();

    // Outer glow
    ctx.beginPath();
    ctx.arc(CENTER, CENTER, RADIUS + 5, 0, Math.PI * 2);
    ctx.strokeStyle = glowColor();
    ctx.lineWidth = 20;
    ctx.stroke();

    // Inner reference circles
    [0.6, 0.3].forEach(function (scale) {
      ctx.beginPath();
      ctx.arc(CENTER, CENTER, RADIUS * scale, 0, Math.PI * 2);
      ctx.strokeStyle = innerRing();
      ctx.lineWidth = 1;
      ctx.setLineDash([4, 8]);
      ctx.stroke();
      ctx.setLineDash([]);
    });

    // Main ring
    ctx.beginPath();
    ctx.arc(CENTER, CENTER, RADIUS, 0, Math.PI * 2);
    ctx.strokeStyle = ringColor();
    ctx.lineWidth = 2.5;
    ctx.stroke();

    // Location markers
    ctx.font = '11px "JetBrains Mono", "SF Mono", "Fira Code", monospace';
    ctx.textAlign = 'center';
    ctx.textBaseline = 'middle';
    ctx.fillStyle = labelColor();
    [0, 0.25, 0.5, 0.75].forEach(function (loc) {
      var a = loc * 2 * Math.PI - Math.PI / 2;
      var x = CENTER + (RADIUS + 22) * Math.cos(a);
      var y = CENTER + (RADIUS + 22) * Math.sin(a);
      ctx.fillText(loc.toFixed(2), x, y);
    });
  }

  // --- Draw connections ---
  function drawConnections(ctx, peers, connections) {
    ctx.lineWidth = 0.7;
    ctx.lineCap = 'round';
    var dark = isDark();
    var alpha = dark ? 0.09 : 0.05;

    ctx.strokeStyle = dark ? 'rgba(0, 127, 255, ' + alpha + ')' : 'rgba(0, 80, 180, ' + alpha + ')';
    ctx.beginPath();
    for (var i = 0; i < connections.length; i++) {
      var c = connections[i];
      var p1 = peers[c[0]].pos;
      var p2 = peers[c[1]].pos;
      // Curved connections through center (like the dashboard)
      var cp = controlPoint(peers[c[0]].location, peers[c[1]].location);
      ctx.moveTo(p1.x, p1.y);
      ctx.quadraticCurveTo(cp.x, cp.y, p2.x, p2.y);
    }
    ctx.stroke();
  }

  // --- Draw peers ---
  function drawPeers(ctx, peers) {
    var large = peers.length > 60;
    var glowR = large ? 4 : 6;
    var dotR = large ? 2.2 : 3.5;
    var gwGlowR = large ? 6 : 9;
    var gwDotR = large ? 3.5 : 5;

    // Glow pass
    for (var i = 0; i < peers.length; i++) {
      var p = peers[i];
      ctx.beginPath();
      ctx.arc(p.pos.x, p.pos.y, p.isGateway ? gwGlowR : glowR, 0, Math.PI * 2);
      ctx.fillStyle = p.isGateway ? 'rgba(245, 158, 11, 0.25)' : peerGlow();
      ctx.fill();
    }
    // Solid pass
    for (var i = 0; i < peers.length; i++) {
      var p = peers[i];
      ctx.beginPath();
      ctx.arc(p.pos.x, p.pos.y, p.isGateway ? gwDotR : dotR, 0, Math.PI * 2);
      ctx.fillStyle = p.isGateway ? gwColor() : peerColor();
      ctx.fill();
    }
  }

  // --- Particle system ---
  var particles = [];
  var lastSpawnTime = 0;

  function spawnParticles(flows, peers, now) {
    // Spawn at a steady rate based on wall time, independent of loop position.
    // This avoids any gap at the loop boundary.
    if (now - lastSpawnTime < SPAWN_INTERVAL) return;
    lastSpawnTime = now;
    if (particles.length >= MAX_PARTICLES) return;

    // Pick the next flow based on where we are in the loop
    var elapsed = (now - loopStart) % LOOP_DURATION;
    // Find the flow closest to current elapsed time
    var bestIdx = 0;
    var bestDist = Infinity;
    for (var i = 0; i < flows.length; i++) {
      var d = Math.abs(flows[i].offsetMs - elapsed);
      // Also check wrap-around distance
      var dWrap = LOOP_DURATION - d;
      var dist = Math.min(d, dWrap);
      if (dist < bestDist) { bestDist = dist; bestIdx = i; }
    }

    // Jitter: pick from a small window around bestIdx for variety
    var jitter = Math.floor(pseudoRandFromTime(now) * 5);
    var idx = (bestIdx + jitter) % flows.length;
    var f = flows[idx];

    var fromPos = peers[f.from].pos;
    var toPos = peers[f.to].pos;
    var dx = toPos.x - fromPos.x;
    var dy = toPos.y - fromPos.y;
    if (dx * dx + dy * dy < 100) return;

    particles.push({
      fromPos: fromPos,
      toPos: toPos,
      cp: controlPoint(peers[f.from].location, peers[f.to].location),
      color: f.color,
      style: f.style,
      startTime: now,
      duration: PARTICLE_DURATION,
    });
  }

  var loopStart = 0;

  // Simple hash for jitter so it's deterministic per-frame but varied
  function pseudoRandFromTime(t) {
    var x = Math.sin(t * 0.001) * 10000;
    return x - Math.floor(x);
  }

  function drawParticles(ctx, now) {
    // Remove expired
    var w = 0;
    for (var i = 0; i < particles.length; i++) {
      if (now - particles[i].startTime <= particles[i].duration) {
        particles[w++] = particles[i];
      }
    }
    particles.length = w;
    if (particles.length === 0) return;

    ctx.save();
    ctx.lineCap = 'round';

    // Batch by color
    var byColor = {};
    for (var i = 0; i < particles.length; i++) {
      var p = particles[i];
      if (!byColor[p.color]) byColor[p.color] = [];
      byColor[p.color].push(p);
    }

    var colors = Object.keys(byColor);
    for (var ci = 0; ci < colors.length; ci++) {
      var color = colors[ci];
      var batch = byColor[color];
      ctx.strokeStyle = color;
      ctx.fillStyle = color;

      for (var pi = 0; pi < batch.length; pi++) {
        var p = batch[pi];
        var t = (now - p.startTime) / p.duration;
        var alpha = 1 - t * 0.5;
        var eased = 1 - (1 - t) * (1 - t);

        var pt = quadBezier(p.fromPos, p.cp, p.toPos, eased);

        // Trail
        var trailT = Math.max(0, eased - 0.15);
        var trailPt = quadBezier(p.fromPos, p.cp, p.toPos, trailT);
        ctx.globalAlpha = alpha * 0.25;
        ctx.lineWidth = 1.5;
        ctx.beginPath();
        ctx.moveTo(trailPt.x, trailPt.y);
        ctx.lineTo(pt.x, pt.y);
        ctx.stroke();

        // Shape
        ctx.globalAlpha = alpha;
        if (p.style === 'diamond') {
          var s = 3.2;
          ctx.save();
          ctx.translate(pt.x, pt.y);
          ctx.rotate(Math.PI / 4);
          ctx.fillRect(-s / 2, -s / 2, s, s);
          ctx.restore();
        } else if (p.style === 'triangle') {
          var dx2 = pt.x - trailPt.x;
          var dy2 = pt.y - trailPt.y;
          var angle = Math.atan2(dy2, dx2);
          var ts = 3.5;
          ctx.save();
          ctx.translate(pt.x, pt.y);
          ctx.rotate(angle);
          ctx.beginPath();
          ctx.moveTo(ts, 0);
          ctx.lineTo(-ts * 0.6, -ts * 0.7);
          ctx.lineTo(-ts * 0.6, ts * 0.7);
          ctx.closePath();
          ctx.fill();
          ctx.restore();
        } else {
          ctx.beginPath();
          ctx.arc(pt.x, pt.y, 2.5, 0, Math.PI * 2);
          ctx.fill();
        }
      }
    }

    ctx.globalAlpha = 1;
    ctx.restore();
  }

  // --- Center label ---
  function drawCenterLabel(ctx) {
    ctx.save();
    ctx.textAlign = 'center';
    ctx.textBaseline = 'middle';
    ctx.font = '11px "Space Grotesk", "Outfit", system-ui, sans-serif';
    ctx.fillStyle = isDark() ? 'rgba(230, 237, 243, 0.25)' : 'rgba(15, 23, 42, 0.2)';
    ctx.fillText('freenet', CENTER, CENTER);
    ctx.restore();
  }

  // --- Main init ---
  function init() {
    var container = document.getElementById('network-hero');
    if (!container) return;

    var canvas = document.createElement('canvas');
    canvas.style.width = '100%';
    canvas.style.maxWidth = SIZE + 'px';
    canvas.style.height = 'auto';
    canvas.style.display = 'block';
    canvas.style.margin = '0 auto';
    container.appendChild(canvas);

    var peers = generatePeers(PEER_COUNT);
    var connections = generateConnections(peers);
    var flows = generateFlows(peers, connections);

    // Handle resize / DPR
    function sizeCanvas() {
      var rect = container.getBoundingClientRect();
      var displayW = Math.min(rect.width, SIZE);
      var dpr = window.devicePixelRatio || 1;
      canvas.width = Math.round(displayW * dpr);
      canvas.height = Math.round(displayW * dpr);
      canvas.style.width = displayW + 'px';
      canvas.style.height = displayW + 'px';
      return { displayW: displayW, dpr: dpr };
    }

    var dims = sizeCanvas();

    // Visibility API: pause when off-screen
    var visible = true;
    var prefersReduced = window.matchMedia('(prefers-reduced-motion: reduce)').matches;

    if (typeof IntersectionObserver !== 'undefined') {
      var observer = new IntersectionObserver(function (entries) {
        visible = entries[0].isIntersecting;
      }, { threshold: 0.1 });
      observer.observe(container);
    }

    document.addEventListener('visibilitychange', function () {
      if (document.hidden) visible = false;
      else visible = true;
    });

    loopStart = performance.now();

    // Static frame for reduced-motion
    if (prefersReduced) {
      var ctx = canvas.getContext('2d');
      var scale = dims.displayW / SIZE;
      ctx.setTransform(dims.dpr * scale, 0, 0, dims.dpr * scale, 0, 0);
      drawRing(ctx);
      drawConnections(ctx, peers, connections);
      drawPeers(ctx, peers);
      drawCenterLabel(ctx);
      return;
    }

    // Animation loop — 60fps for smooth particles

    // Offscreen canvas for static elements (ring, connections, peers, stats)
    var staticCanvas = document.createElement('canvas');
    var staticDirty = true;
    var lastDark = isDark();

    function renderStatic() {
      var dpr = dims.dpr;
      var w = Math.round(dims.displayW * dpr);
      if (staticCanvas.width !== w || staticCanvas.height !== w) {
        staticCanvas.width = w;
        staticCanvas.height = w;
      }
      var sctx = staticCanvas.getContext('2d');
      var scale = dims.displayW / SIZE;
      sctx.setTransform(dpr * scale, 0, 0, dpr * scale, 0, 0);
      sctx.clearRect(0, 0, SIZE, SIZE);
      drawRing(sctx);
      drawConnections(sctx, peers, connections);
      drawPeers(sctx, peers);
      drawCenterLabel(sctx);
      staticDirty = false;
      lastDark = isDark();
    }

    function frame(now) {
      requestAnimationFrame(frame);
      if (!visible) return;

      // Rebuild static layer only when needed (theme change, peer count update, resize)
      if (staticDirty || isDark() !== lastDark) {
        renderStatic();
      }

      var ctx = canvas.getContext('2d');
      var dpr = dims.dpr;
      var w = Math.round(dims.displayW * dpr);
      if (canvas.width !== w || canvas.height !== w) {
        canvas.width = w;
        canvas.height = w;
        staticDirty = true;
        renderStatic();
      }

      // Blit static layer
      ctx.setTransform(1, 0, 0, 1, 0, 0);
      ctx.clearRect(0, 0, canvas.width, canvas.height);
      ctx.drawImage(staticCanvas, 0, 0);

      // Draw particles in logical coordinates
      var scale = dims.displayW / SIZE;
      ctx.setTransform(dpr * scale, 0, 0, dpr * scale, 0, 0);
      spawnParticles(flows, peers, now);
      drawParticles(ctx, now);
    }

    // Mark static dirty on resize
    var origResize = window.onresize;
    window.addEventListener('resize', function () { dims = sizeCanvas(); staticDirty = true; });

    requestAnimationFrame(frame);
  }

  // Start when DOM ready
  if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', init);
  } else {
    init();
  }
})();
