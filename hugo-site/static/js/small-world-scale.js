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
    let numPeers = 5;
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
        peers = d3.range(numPeers).map(i => {
            const angle = (i / numPeers) * 2 * Math.PI;
            return {
                x: width / 2 + radius * Math.cos(angle),
                y: height / 2 + radius * Math.sin(angle),
                index: i
            };
        });

        // Initialize links array
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
        const connectionsPerNode = (links.length * 2 / numPeers).toFixed(1);
        ctx.fillText(`Network Size: ${numPeers} nodes, Avg. Connections/Node: ${connectionsPerNode}`, width/2, height - 10);
    }

    function calculateAveragePathLength() {
        // More reference nodes for smaller networks
        const numReferenceNodes = Math.min(10, Math.ceil(numPeers * 0.2));
        const referenceNodes = [];
        const stride = Math.floor(peers.length / numReferenceNodes);
        
        // Select evenly spaced reference nodes
        for (let i = 0; i < numReferenceNodes; i++) {
            referenceNodes.push(peers[i * stride]);
        }
        
        // Increased sampling for better averages
        const samplesPerRef = Math.min(
            Math.ceil(numPeers * 0.9), // Sample up to 90% of nodes
            90 // but cap at 90 samples per reference node
        );
        
        let totalLength = 0;
        let pathCount = 0;
        
        // Calculate paths from reference nodes to sampled targets
        for (const refNode of referenceNodes) {
            for (let i = 0; i < samplesPerRef; i++) {
                const targetIdx = Math.floor(Math.random() * peers.length);
                const target = peers[targetIdx];
                if (refNode !== target) {
                    const path = findShortestPath(refNode, target);
                    if (path) {
                        totalLength += path.length - 1;
                        pathCount++;
                    }
                }
            }
        }

        return pathCount > 0 ? totalLength / pathCount : 0;
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

            // Find neighbors
            const neighbors = links.reduce((acc, link) => {
                if (link.source === node && !visited.has(link.target.index)) {
                    acc.push(link.target);
                } else if (link.target === node && !visited.has(link.source.index)) {
                    acc.push(link.source);
                }
                return acc;
            }, []);

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
            .domain([0, maxPeers])
            .range([0, chartWidth]);

        const y = d3.scaleLinear()
            .domain([0, d3.max(averagePathLengths, d => d.pathLength) * 1.1])
            .range([chartHeight, 0]);

        // Add gridlines
        g.append('g')
            .attr('class', 'grid')
            .attr('opacity', 0.1)
            .call(d3.axisLeft(y)
                .ticks(10)
                .tickSize(-chartWidth)
                .tickFormat(''));

        g.append('g')
            .attr('class', 'grid')
            .attr('transform', `translate(0,${chartHeight})`)
            .attr('opacity', 0.1)
            .call(d3.axisBottom(x)
                .ticks(10)
                .tickSize(chartHeight)
                .tickFormat(''));

        // Add the axes
        g.append('g')
            .attr('transform', `translate(0,${chartHeight})`)
            .call(d3.axisBottom(x)
                .ticks(10)
                .tickFormat(d => d === 0 ? '0' : d));

        g.append('g')
            .call(d3.axisLeft(y)
                .ticks(10));

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
                    
                    // Very fine granularity for small networks, increasing with size
                    const stepSize = numPeers < 20 ? 2 :
                                   numPeers < 50 ? 4 :
                                   numPeers < 100 ? 10 : 
                                   numPeers < 200 ? 20 :
                                   numPeers < 350 ? 30 : 40;
                    numPeers += stepSize;
                    
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
        pathLength: initialAvgPathLength,
        connectionsPerNode: links.length * 2 / numPeers
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
