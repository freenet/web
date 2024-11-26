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
    const canvas = document.getElementById('networkCanvas3');
    const ctx = canvas.getContext('2d');
    const width = canvas.width;
    const height = canvas.height;

    // Parameters
    let numPeers = 30;
    const maxPeers = 10000;
    const radius = Math.min(width, height) * 0.4;
    const connectionProbability = (distance) => 1 / (distance + 1);
    
    let peers = [];
    let links = [];
    let averagePathLengths = [];
    let animationFrame;
    let isSimulating = false;

    function initializeNetwork() {
        peers = d3.range(numPeers).map(i => {
            const angle = (i / numPeers) * 2 * Math.PI;
            return {
                x: width / 2 + radius * Math.cos(angle),
                y: height / 2 + radius * Math.sin(angle),
                index: i
            };
        });

        links = [];
        for (let i = 0; i < numPeers; i++) {
            const nextIndex = (i + 1) % numPeers;
            links.push({ source: peers[i], target: peers[nextIndex] });
            
            for (let j = i + 2; j < numPeers; j++) {
                const distance = Math.min(Math.abs(i - j), numPeers - Math.abs(i - j));
                const prob = connectionProbability(distance);
                if (Math.random() < prob) {
                    links.push({ source: peers[i], target: peers[j] });
                }
            }
        }
    }

    function draw() {
        ctx.clearRect(0, 0, width, height);

        // Always show visualization but limit rendered nodes
        const renderLimit = 200;
        const renderStep = Math.max(1, Math.floor(peers.length / renderLimit));
        
        // Draw a subset of links
        ctx.strokeStyle = 'rgba(0, 127, 255, 0.3)';
        links.forEach((link, i) => {
            if (i % renderStep === 0) {
                ctx.beginPath();
                ctx.moveTo(link.source.x, link.source.y);
                ctx.lineTo(link.target.x, link.target.y);
                ctx.stroke();
            }
        });

        // Draw a subset of nodes
        ctx.fillStyle = '#007FFF';
        peers.forEach((peer, i) => {
            if (i % renderStep === 0) {
                ctx.beginPath();
                ctx.arc(peer.x, peer.y, 3, 0, 2 * Math.PI);
                ctx.fill();
            }
        });

        // Show network size
        ctx.fillStyle = '#007FFF';
        ctx.textAlign = 'center';
        ctx.font = '14px Arial';
        ctx.fillText(`Network Size: ${numPeers} nodes`, width/2, height - 10);
    }

    // Pre-computed network statistics for common sizes
    const networkStats = new Map();
    
    function calculateAveragePathLength() {
        // Use cached value if available
        if (networkStats.has(numPeers)) {
            return networkStats.get(numPeers);
        }

        // Fixed sample size regardless of network size
        const sampleSize = 50;
        const sampleNodes = selectRepresentativeNodes(sampleSize);
        
        let totalLength = 0;
        let pathCount = 0;

        // Calculate paths between sampled pairs
        for (let i = 0; i < sampleNodes.length; i++) {
            for (let j = i + 1; j < sampleNodes.length; j++) {
                const pathLength = findGreedyPath(sampleNodes[i], sampleNodes[j], sampleNodes);
                if (pathLength !== Infinity) {
                    totalLength += pathLength;
                    pathCount++;
                }
            }
        }

        const avgPathLength = pathCount > 0 ? totalLength / pathCount : 0;
        
        // Cache the result
        networkStats.set(numPeers, avgPathLength);
        
        return avgPathLength;
    }

    function selectRepresentativeNodes(sampleSize) {
        // Select nodes that are evenly distributed around the ring
        const step = Math.max(1, Math.floor(peers.length / sampleSize));
        return peers.filter((_, index) => index % step === 0).slice(0, sampleSize);
    }

    function findGreedyPath(start, target, nodes) {
        const visited = new Set();
        let current = start;
        let path = [current];
        
        while (current !== target) {
            visited.add(current);
            
            // Find all neighbors of current node
            let neighbors = links
                .filter(link => 
                    (link.source === current || link.target === current) &&
                    !visited.has(link.source === current ? link.target : link.source)
                )
                .map(link => link.source === current ? link.target : link.source);
            
            if (neighbors.length === 0) {
                return Infinity; // No path found
            }
            
            // Choose the neighbor closest to target (in terms of ring distance)
            let nextNode = neighbors.reduce((best, neighbor) => {
                let distToTarget = Math.min(
                    Math.abs(neighbor.index - target.index),
                    nodes.length - Math.abs(neighbor.index - target.index)
                );
                let bestDist = Math.min(
                    Math.abs(best.index - target.index),
                    nodes.length - Math.abs(best.index - target.index)
                );
                return distToTarget < bestDist ? neighbor : best;
            });
            
            current = nextNode;
            path.push(current);
            
            if (path.length > nodes.length) {
                return Infinity; // Prevent infinite loops
            }
        }
        
        return path.length - 1; // Return path length (number of hops)
    }

    function updateChart() {
        const margin = {top: 20, right: 20, bottom: 30, left: 40};
        const chartWidth = width - margin.left - margin.right;
        const chartHeight = height - margin.top - margin.bottom;

        // Create SVG if it doesn't exist
        let svg = d3.select('#scalingChart svg');
        if (svg.empty()) {
            svg = d3.select('#scalingChart')
                .append('svg')
                .attr('width', width)
                .attr('height', height);
        }
        svg.selectAll('*').remove();

        const g = svg.append('g')
            .attr('transform', `translate(${margin.left},${margin.top})`);

        const x = d3.scaleLinear()
            .domain([30, maxPeers])
            .range([0, chartWidth]);

        const y = d3.scaleLinear()
            .domain([0, d3.max(averagePathLengths, d => d.pathLength) * 1.1])
            .range([chartHeight, 0]);

        // Add the axes
        g.append('g')
            .attr('transform', `translate(0,${chartHeight})`)
            .call(d3.axisBottom(x));

        g.append('g')
            .call(d3.axisLeft(y));

        // Add axis labels
        g.append('text')
            .attr('transform', `translate(${chartWidth/2},${chartHeight + margin.bottom})`)
            .style('text-anchor', 'middle')
            .text('Network Size');

        g.append('text')
            .attr('transform', 'rotate(-90)')
            .attr('y', 0 - margin.left)
            .attr('x', 0 - (chartHeight / 2))
            .attr('dy', '1em')
            .style('text-anchor', 'middle')
            .text('Average Path Length');

        // Plot points
        g.selectAll('circle')
            .data(averagePathLengths)
            .enter()
            .append('circle')
            .attr('cx', d => x(d.numPeers))
            .attr('cy', d => y(d.pathLength))
            .attr('r', 3)
            .style('fill', '#007FFF'); // Primary blue

        // Add connecting line
        const line = d3.line()
            .x(d => x(d.numPeers))
            .y(d => y(d.pathLength));

        g.append('path')
            .datum(averagePathLengths)
            .attr('fill', 'none')
            .attr('stroke', '#007FFF') // Primary blue
            .attr('stroke-width', 1.5)
            .attr('d', line);
    }

    function simulate() {
        if (!isSimulating) {
            isSimulating = true;
            const btn = document.getElementById('scalePlayPauseBtn');
            const icon = btn.querySelector('i');
            icon.className = 'fas fa-pause';
            
            async function step() {
                if (!isSimulating) return;
                
                // Initialize and draw with small delay to allow UI updates
                initializeNetwork();
                await new Promise(resolve => setTimeout(resolve, 10));
                draw();
                
                const avgPathLength = calculateAveragePathLength();
                averagePathLengths.push({
                    numPeers: numPeers,
                    pathLength: avgPathLength
                });
                updateChart();
                
                // Adjust step size based on network phase
                const stepSize = numPeers < 100 ? 10 : 
                               numPeers < 500 ? 25 :
                               numPeers < 1000 ? 100 : 200;
                numPeers += stepSize;
                
                if (numPeers <= maxPeers) {
                    // Longer delays for computation-heavy phases
                    const delay = numPeers < 100 ? 400 : 
                                numPeers < 500 ? 600 :
                                numPeers < 1000 ? 1000 : 1500;
                    setTimeout(step, delay);
                } else {
                    isSimulating = false;
                    const btn = document.getElementById('scalePlayPauseBtn');
                    const icon = btn.querySelector('i');
                    icon.className = 'fas fa-play';
                }
            }
            
            step().catch(console.error);
        } else {
            isSimulating = false;
            const btn = document.getElementById('scalePlayPauseBtn');
            const icon = btn.querySelector('i');
            icon.className = 'fas fa-play';
            cancelAnimationFrame(animationFrameId);
        }
    }

    // Initialize network and add button handlers
    initializeNetwork();
    draw();
    
    // Initialize the graph with the first data point
    const initialAvgPathLength = calculateAveragePathLength();
    averagePathLengths = [{
        numPeers: numPeers,
        pathLength: initialAvgPathLength
    }];
    updateChart();
    
    const playPauseBtn = document.getElementById('scalePlayPauseBtn');
    const resetBtn = document.getElementById('resetScaleBtn');
    
    playPauseBtn.addEventListener('click', simulate);
    resetBtn.addEventListener('click', reset);

    function reset() {
        cancelAnimationFrame(animationFrame);
        isSimulating = false;
        numPeers = 30;
        averagePathLengths = [{
            numPeers: numPeers,
            pathLength: calculateAveragePathLength()
        }];
        initializeNetwork();
        draw();
        updateChart();
        
        const btn = document.getElementById('scalePlayPauseBtn');
        const icon = btn.querySelector('i');
        icon.className = 'fas fa-play';
    }
    
});
