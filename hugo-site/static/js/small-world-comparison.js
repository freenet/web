// Wait for D3.js to be available
function waitForD3() {
    return new Promise((resolve) => {
        if (typeof d3 !== 'undefined') {
            resolve();
        } else {
            const script = document.createElement('script');
            script.src = "https://d3js.org/d3.v7.min.js";
            script.onload = () => resolve();
            document.head.appendChild(script);
        }
    });
}

waitForD3().then(() => {
    const numNodes = 400;
    const maxHops = 30;
    let isPlaying = false;
    let animationFrame;
    
    // Setup canvases
    const smallWorldCanvas = document.getElementById('smallWorldCanvas');
    const randomCanvas = document.getElementById('randomNetworkCanvas');
    const swCtx = smallWorldCanvas.getContext('2d');
    const rnCtx = randomCanvas.getContext('2d');
    
    // Network state
    let smallWorldNetwork = { nodes: [], links: [], stats: { success: 0, attempts: 0, totalPathLength: 0 } };
    let randomNetwork = { nodes: [], links: [], stats: { success: 0, attempts: 0, totalPathLength: 0 } };
    
    function initializeNetworks() {
        // Reset stats
        smallWorldNetwork.stats = { success: 0, attempts: 0, totalPathLength: 0 };
        randomNetwork.stats = { success: 0, attempts: 0, totalPathLength: 0 };
        
        // Create nodes in a ring layout
        const radius = Math.min(smallWorldCanvas.width, smallWorldCanvas.height) * 0.315; // 0.45 * 0.7 = 0.315 (30% smaller)
        const nodes = d3.range(numNodes).map(i => {
            const angle = (i / numNodes) * 2 * Math.PI;
            return {
                id: i,
                x: smallWorldCanvas.width/2 + radius * Math.cos(angle),
                y: smallWorldCanvas.height/2 + radius * Math.sin(angle)
            };
        });
        
        // Initialize both networks with the same nodes
        smallWorldNetwork.nodes = JSON.parse(JSON.stringify(nodes));
        randomNetwork.nodes = JSON.parse(JSON.stringify(nodes));
        
        // Create small world network links
        smallWorldNetwork.links = createSmallWorldLinks();
        
        // Create random network links with same average degree
        const avgDegree = (smallWorldNetwork.links.length * 2) / numNodes;
        randomNetwork.links = createRandomLinks(avgDegree);
        
        updateStats();
    }
    
    function createSmallWorldLinks() {
        const links = [];
        const k = 8; // Each node connects to k nearest neighbors
        const beta = 0.3; // Probability of rewiring
        
        // First ensure each node is connected to its immediate neighbors
        for (let i = 0; i < numNodes; i++) {
            // Connect to previous node
            const prev = (i - 1 + numNodes) % numNodes;
            links.push({
                source: i,
                target: prev,
                rewired: false
            });
            
            // Connect to next node
            const next = (i + 1) % numNodes;
            links.push({
                source: i,
                target: next,
                rewired: false
            });
        }
        
        // Add additional longer-range connections
        for (let i = 0; i < numNodes; i++) {
            for (let j = 2; j <= k/2; j++) {
                const target = (i + j) % numNodes;
                const link = {
                    source: i,
                    target: target,
                    rewired: false
                };
                
                // Only rewire non-neighbor connections
                if (Math.random() < beta) {
                    let newTarget;
                    do {
                        newTarget = Math.floor(Math.random() * numNodes);
                    } while (
                        newTarget === link.source || 
                        newTarget === ((i + 1) % numNodes) || // Don't rewire to immediate neighbors
                        newTarget === ((i - 1 + numNodes) % numNodes) ||
                        links.some(l => 
                            (l.source === link.source && l.target === newTarget) ||
                            (l.source === newTarget && l.target === link.source))
                    );
                    link.target = newTarget;
                    link.rewired = true;
                }
                
                links.push(link);
            }
        }
        
        return links;
    }
    
    function createRandomLinks(avgDegree) {
        const links = [];
        const totalLinks = Math.floor((avgDegree * numNodes) / 2);
        
        while (links.length < totalLinks) {
            const source = Math.floor(Math.random() * numNodes);
            let target;
            do {
                target = Math.floor(Math.random() * numNodes);
            } while (target === source || 
                    links.some(l => 
                        (l.source === source && l.target === target) ||
                        (l.source === target && l.target === source)));
            
            links.push({ source, target });
        }
        
        return links;
    }
    
    function findPath(network, start, end) {
        const path = [start];
        const visited = new Set([start]);
        let current = start;
        let steps = 0;
        
        while (current !== end && steps < maxHops) {
            // Get all unvisited neighbors
            const neighbors = network.links
                .filter(l => l.source === current || l.target === current)
                .map(l => l.source === current ? l.target : l.source)
                .filter(n => !visited.has(n));
            
            if (neighbors.length === 0) {
                return null; // Dead end
            }
            
            // Calculate geometric distances to target
            const distances = neighbors.map(n => {
                const dx = network.nodes[n].x - network.nodes[end].x;
                const dy = network.nodes[n].y - network.nodes[end].y;
                return {
                    node: n,
                    distance: Math.sqrt(dx * dx + dy * dy)
                };
            });
            
            // Choose closest neighbor to target
            const nextNode = distances.reduce((a, b) => 
                a.distance < b.distance ? a : b
            ).node;
            
            current = nextNode;
            path.push(current);
            visited.add(current);
            steps++;
        }
        
        return steps < maxHops ? path : null;
    }
    
    function drawNetwork(ctx, network, path = null) {
        ctx.clearRect(0, 0, ctx.canvas.width, ctx.canvas.height);
        
        // Draw links
        network.links.forEach(link => {
            const source = network.nodes[link.source];
            const target = network.nodes[link.target];
            
            ctx.beginPath();
            ctx.moveTo(source.x, source.y);
            ctx.lineTo(target.x, target.y);
            ctx.strokeStyle = 'rgba(0, 127, 255, 0.1)';
            ctx.stroke();
        });
        
        // Draw path if exists
        if (path) {
            ctx.beginPath();
            ctx.strokeStyle = '#007FFF';
            ctx.lineWidth = 2;
            for (let i = 0; i < path.length - 1; i++) {
                const source = network.nodes[path[i]];
                const target = network.nodes[path[i + 1]];
                ctx.moveTo(source.x, source.y);
                ctx.lineTo(target.x, target.y);
            }
            ctx.stroke();
            ctx.lineWidth = 1;
        }
        
        // Draw nodes
        network.nodes.forEach((node, i) => {
            ctx.beginPath();
            ctx.arc(node.x, node.y, 2, 0, 2 * Math.PI);
            ctx.fillStyle = path && (path[0] === i || path[path.length - 1] === i) 
                ? '#0052cc' 
                : '#007FFF';
            ctx.fill();
        });
    }
    
    function updateStats() {
        const swStats = document.getElementById('smallWorldStats');
        const rnStats = document.getElementById('randomNetworkStats');
        
        const swSuccess = smallWorldNetwork.stats.attempts === 0 ? 0 :
            (smallWorldNetwork.stats.success / smallWorldNetwork.stats.attempts * 100).toFixed(1);
        const swAvgPath = smallWorldNetwork.stats.success === 0 ? 0 :
            (smallWorldNetwork.stats.totalPathLength / smallWorldNetwork.stats.success).toFixed(1);
        
        const rnSuccess = randomNetwork.stats.attempts === 0 ? 0 :
            (randomNetwork.stats.success / randomNetwork.stats.attempts * 100).toFixed(1);
        const rnAvgPath = randomNetwork.stats.success === 0 ? 0 :
            (randomNetwork.stats.totalPathLength / randomNetwork.stats.success).toFixed(1);
        
        swStats.innerHTML = `Success Rate: ${swSuccess}%<br>Avg Path Length: ${swAvgPath}<br>Attempts: ${smallWorldNetwork.stats.attempts}`;
        rnStats.innerHTML = `Success Rate: ${rnSuccess}%<br>Avg Path Length: ${rnAvgPath}<br>Attempts: ${randomNetwork.stats.attempts}`;
    }
    
    async function simulateRouting() {
        if (!isPlaying) return;
        
        // Select random source and target
        const source = Math.floor(Math.random() * numNodes);
        let target;
        do {
            target = Math.floor(Math.random() * numNodes);
        } while (target === source);
        
        // Find paths in both networks
        const swPath = findPath(smallWorldNetwork, source, target);
        const rnPath = findPath(randomNetwork, source, target);
        
        // Update stats
        smallWorldNetwork.stats.attempts++;
        randomNetwork.stats.attempts++;
        if (swPath) {
            smallWorldNetwork.stats.success++;
            smallWorldNetwork.stats.totalPathLength += swPath.length - 1;
        }
        if (rnPath) {
            randomNetwork.stats.success++;
            randomNetwork.stats.totalPathLength += rnPath.length - 1;
        }
        
        // Draw networks with paths
        drawNetwork(swCtx, smallWorldNetwork, swPath);
        drawNetwork(rnCtx, randomNetwork, rnPath);
        updateStats();
        
        // Wait before next attempt
        await new Promise(resolve => setTimeout(resolve, 1000));
        
        if (isPlaying) {
            animationFrame = requestAnimationFrame(() => simulateRouting());
        }
    }
    
    function togglePlayPause() {
        isPlaying = !isPlaying;
        const btn = document.getElementById('comparisonPlayPauseBtn');
        const icon = btn.querySelector('i');
        icon.className = isPlaying ? 'fas fa-pause' : 'fas fa-play';
        
        if (isPlaying) {
            simulateRouting();
        }
    }
    
    function reset() {
        isPlaying = false;
        const btn = document.getElementById('comparisonPlayPauseBtn');
        const icon = btn.querySelector('i');
        icon.className = 'fas fa-play';
        
        if (animationFrame) {
            cancelAnimationFrame(animationFrame);
        }
        
        initializeNetworks();
        drawNetwork(swCtx, smallWorldNetwork);
        drawNetwork(rnCtx, randomNetwork);
    }
    
    // Initialize
    document.getElementById('comparisonPlayPauseBtn').addEventListener('click', togglePlayPause);
    document.getElementById('resetComparisonBtn').addEventListener('click', reset);
    
    initializeNetworks();
    drawNetwork(swCtx, smallWorldNetwork);
    drawNetwork(rnCtx, randomNetwork);
});