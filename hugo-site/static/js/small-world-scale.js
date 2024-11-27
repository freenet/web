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
    let numPeers = 3;
    const maxPeers = 500;
    const radius = Math.min(width, height) * 0.4;
    // Target ~4 long-range connections per peer (plus 2 ring connections)
    const connectionProbability = (distance, networkSize) => {
        // We want each node to make ~4 long-range connections
        const targetLongRangePerNode = 4;
        
        // Calculate probability needed to achieve target connections
        // Each node attempts (n-3) connections after ring connections
        // We want targetLongRangePerNode successes out of these attempts
        const baseProb = targetLongRangePerNode / (networkSize - 3);
        
        // Adjust for distance preference (closer nodes more likely)
        const distanceWeight = 1 / (distance + 1);
        const normalizedWeight = distanceWeight / (1 + Math.log(networkSize));
        
        return Math.min(1, baseProb * normalizedWeight * networkSize);
    };
    
    let peers = [];
    let links = [];
    let averagePathLengths = [];
    let animationFrame;
    let isSimulating = false;

    function initializeNetwork() {
        buildAdjacencyLists(); // Clear old adjacency lists
        
        // Pre-calculate sine and cosine values
        const angleStep = (2 * Math.PI) / numPeers;
        const centerX = width / 2;
        const centerY = height / 2;
        
        peers = new Array(numPeers);
        for (let i = 0; i < numPeers; i++) {
            const angle = i * angleStep;
            peers[i] = {
                x: centerX + radius * Math.cos(angle),
                y: centerY + radius * Math.sin(angle),
                index: i
            };
        }

        links = [];
        
        // Create ring connections and store existing connections for quick lookup
        const existingConnections = new Set();
        for (let i = 0; i < numPeers; i++) {
            // Connect to immediate neighbors (ring structure)
            const nextIndex = (i + 1) % numPeers;
            links.push({ source: peers[i], target: peers[nextIndex] });
            existingConnections.add(`${i}-${nextIndex}`);
            existingConnections.add(`${nextIndex}-${i}`);
        }

        // Add long-range connections
        for (let i = 0; i < numPeers; i++) {
            for (let j = 0; j < numPeers; j++) {
                if (i !== j) {
                    // Skip if connection already exists
                    if (existingConnections.has(`${i}-${j}`)) continue;
                    
                    // Calculate shortest distance on the ring
                    const distance = Math.min(
                        Math.abs(i - j),
                        numPeers - Math.abs(i - j)
                    );
                    
                    // Probability decreases with distance
                    const prob = connectionProbability(distance, numPeers);
                    
                    if (Math.random() < prob) {
                        links.push({ source: peers[i], target: peers[j] });
                        existingConnections.add(`${i}-${j}`);
                        existingConnections.add(`${j}-${i}`);
                    }
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

        // Show network size and connection count
        ctx.fillStyle = '#007FFF';
        ctx.textAlign = 'center';
        ctx.font = '14px Arial';
        ctx.fillText(`Network Size: ${numPeers} nodes`, width/2, height - 10);
    }

    function calculateAveragePathLength() {
        if (numPeers <= 1) return 0;
        
        buildAdjacencyLists(); // Ensure adjacency lists are up to date
        
        let totalLength = 0;
        let pathCount = 0;
        
        // For small networks, check all pairs
        if (numPeers <= 20) {
            for (let i = 0; i < numPeers; i++) {
                for (let j = i + 1; j < numPeers; j++) {
                    const path = findShortestPath(peers[i], peers[j]);
                    if (path) {
                        totalLength += path.length - 1;
                        pathCount++;
                    }
                }
            }
        } else {
            // For larger networks, sample pairs
            const numSamples = Math.min(200, numPeers * 2);
            for (let i = 0; i < numSamples; i++) {
                const source = Math.floor(Math.random() * numPeers);
                let target;
                do {
                    target = Math.floor(Math.random() * numPeers);
                } while (target === source);
                
                const path = findShortestPath(peers[source], peers[target]);
                if (path) {
                    totalLength += path.length - 1;
                    pathCount++;
                }
            }
        }
        
        return pathCount > 0 ? totalLength / pathCount : 0;
    }

    // Pre-compute adjacency lists for faster path finding
    let adjacencyLists = new Map();
    
    function buildAdjacencyLists() {
        adjacencyLists.clear();
        for (const peer of peers) {
            adjacencyLists.set(peer, []);
        }
        for (const link of links) {
            adjacencyLists.get(link.source).push(link.target);
            adjacencyLists.get(link.target).push(link.source);
        }
    }

    function findShortestPath(start, end) {
        if (start === end) return [start];
        
        const queue = [start];
        let queueStart = 0;
        const visited = new Set([start.index]);
        const previous = new Map();
        
        while (queueStart < queue.length) {
            const node = queue[queueStart++];
            
            if (node === end) {
                // Reconstruct path
                const path = [end];
                let current = end;
                while (previous.has(current)) {
                    current = previous.get(current);
                    path.unshift(current);
                }
                return path;
            }

            // Use pre-computed adjacency list
            const neighbors = adjacencyLists.get(node) || [];
            for (const neighbor of neighbors) {
                if (!visited.has(neighbor.index)) {
                    visited.add(neighbor.index);
                    previous.set(neighbor, node);
                    queue.push(neighbor);
                }
            }
        }

        return null;
    }

    // Keep SVG reference to avoid repeated selections
    let chartSvg;
    
    // Cache DOM elements and D3 scales
    let chartG;
    let xScale, yScale;
    let pathElement, pointElements;
    
    function initializeChart() {
        const margin = {top: 30, right: 40, bottom: 90, left: 60};
        const chartWidth = width - margin.left - margin.right;
        const chartHeight = height - margin.top - margin.bottom;

        // Initialize scales first
        xScale = d3.scaleLinear()
            .domain([0, maxPeers])
            .range([0, chartWidth]);
            
        yScale = d3.scaleLinear()
            .domain([0, 3])  // Initial domain, will be updated
            .range([chartHeight, 0]);

        if (!chartSvg) {
            chartSvg = d3.select('#scalingChart svg')
                .attr('width', width)
                .attr('height', height)
                .style('overflow', 'visible'); // Allow elements to render outside SVG bounds
                
            chartG = chartSvg.append('g')
                .attr('transform', `translate(${margin.left},${margin.top})`);
                
            // Add static elements
            // Add gridlines
            // Add horizontal grid lines
            chartG.append('g')
                .attr('class', 'grid')
                .attr('opacity', 0.1)
                .call(d3.axisLeft(yScale)
                    .ticks(10)
                    .tickSize(-chartWidth)
                    .tickFormat(''));

            // Add vertical grid lines
            chartG.append('g')
                .attr('class', 'grid')
                .attr('transform', `translate(0,${chartHeight})`)
                .attr('opacity', 0.1)
                .call(d3.axisBottom(xScale)
                    .ticks(10)
                    .tickSize(-chartHeight)  // Changed from positive to negative
                    .tickFormat(''));

            // Add the axes
            chartG.append('g')
                .attr('transform', `translate(0,${chartHeight})`)
                .attr('class', 'x-axis')
                .call(d3.axisBottom(xScale)
                    .ticks(10)
                    .tickFormat(d => d === 0 ? '0' : d));

            chartG.append('g')
                .attr('class', 'y-axis')
                .call(d3.axisLeft(yScale)
                    .ticks(10));

            // Add axis labels
            chartG.append('text')
                .attr('transform', `translate(${chartWidth/2},${chartHeight + 45})`)
                .style('text-anchor', 'middle')
                .text('Network Size');

            chartG.append('text')
                .attr('transform', 'rotate(-90)')
                .attr('y', 0 - margin.left + 15)
                .attr('x', 0 - (chartHeight / 2))
                .attr('dy', '1em')
                .style('text-anchor', 'middle')
                .text('Average Path Length');
        }
        
        xScale = d3.scaleLinear()
            .domain([0, maxPeers])
            .range([0, chartWidth]);
            
        yScale = d3.scaleLinear()
            .domain([0, 3])  // Initial domain, will be updated
            .range([chartHeight, 0]);
            
        // Create elements that will be updated
        pathElement = chartG.append('path')
            .attr('fill', 'none')
            .attr('stroke', '#007FFF')
            .attr('stroke-width', 1.5);
            
        pointElements = chartG.append('g')
            .attr('class', 'points');
    }
    
    function updateChart() {
        // Update scales
        yScale.domain([0, Math.max(3, d3.max(averagePathLengths, d => d.pathLength) * 1.1)]);
        
        // Update line
        const line = d3.line()
            .x(d => xScale(d.numPeers))
            .y(d => yScale(d.pathLength));
            
        pathElement.datum(averagePathLengths)
            .attr('d', line);
            
        // Update points
        const points = pointElements.selectAll('circle')
            .data(averagePathLengths);
            
        points.enter()
            .append('circle')
            .merge(points)
            .attr('cx', d => xScale(d.numPeers))
            .attr('cy', d => yScale(d.pathLength))
            .attr('r', 3)
            .style('fill', '#007FFF');
            
        points.exit().remove();

    }

    function simulate() {
        if (!isSimulating) {
            isSimulating = true;
            const btn = document.getElementById('scalePlayPauseBtn');
            const icon = btn.querySelector('i');
            icon.className = 'fas fa-pause';
            
            async function step() {
                if (!isSimulating) return;
                
                const batchSize = 3; // Process multiple steps at once
                for (let i = 0; i < batchSize && numPeers <= maxPeers; i++) {
                    initializeNetwork();
                    draw();
                    
                    const avgPathLength = calculateAveragePathLength();
                    averagePathLengths.push({
                        numPeers: numPeers,
                        pathLength: avgPathLength,
                        connectionsPerNode: links.length * 2 / numPeers
                    });
                    
                    // Smaller step sizes for more gradual growth
                    const stepSize = numPeers < 20 ? 1 :
                                   numPeers < 50 ? 2 :
                                   numPeers < 100 ? 5 : 
                                   numPeers < 200 ? 10 : 15;
                    numPeers += stepSize;
        
                    // Limit array size to prevent memory growth
                    if (averagePathLengths.length > 100) {
                        const keepEvery = Math.ceil(averagePathLengths.length / 100);
                        averagePathLengths = averagePathLengths.filter((_, i) => i % keepEvery === 0);
                    }
                    
                    // Allow UI updates between iterations
                    if (i < batchSize - 1) {
                        await new Promise(resolve => setTimeout(resolve, 0));
                    }
                }
                
                updateChart();
                
                if (numPeers <= maxPeers) {
                    const delay = numPeers < 20 ? 500 :
                                numPeers < 50 ? 400 :
                                numPeers < 100 ? 300 : 
                                numPeers < 250 ? 250 :
                                200; // More time studying small networks
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
            if (animationFrame) {
                cancelAnimationFrame(animationFrame);
            }
        }
    }

    // Initialize network and add button handlers
    initializeNetwork();
    buildAdjacencyLists();
    draw();
    initializeChart();
    
    // Initialize chart first
    updateChart();
    
    // Then add initial data points
    const initialAvgPathLength = calculateAveragePathLength();
    averagePathLengths = [
        {
            numPeers: numPeers,
            pathLength: initialAvgPathLength,
            connectionsPerNode: links.length * 2 / numPeers
        }
    ];
    
    const playPauseBtn = document.getElementById('scalePlayPauseBtn');
    const resetBtn = document.getElementById('resetScaleBtn');
    
    playPauseBtn.addEventListener('click', simulate);
    resetBtn.addEventListener('click', reset);

    function reset() {
        cancelAnimationFrame(animationFrame);
        isSimulating = false;
        numPeers = 3;
        averagePathLengths = [
            {
                numPeers: 0,
                pathLength: 0,
                connectionsPerNode: 0
            },
            {
                numPeers: numPeers,
                pathLength: calculateAveragePathLength(),
                connectionsPerNode: links.length * 2 / numPeers
            }
        ];
        initializeNetwork();
        draw();
        updateChart();
        
        const btn = document.getElementById('scalePlayPauseBtn');
        const icon = btn.querySelector('i');
        icon.className = 'fas fa-play';
    }
    
});
